mod config;

use config::Config;
use fingr::{Fngr, prelude::*};
use fingr::{
    networking::{Action, JSONResponse, Request, Response, ResponseStatus},
    userlist::{JSONStatus, Status, UserList},
};
use std::{path::PathBuf, sync::Arc, time::Duration};
use tokio::{
    fs::{File, OpenOptions},
    io::BufStream,
    net::TcpListener,
    sync::Mutex,
    time::{Instant, sleep},
};

// struct holds the state of the server
struct Server {
    config: Config,
    #[allow(unused)]
    // lock: Option<File>,
    users: UserList,
}

impl Fngr for Server {
    type SelfLock = Arc<Mutex<Self>>;

    async fn login(state: Arc<Mutex<Self>>, req: Request) -> Result<Response> {
        Self::change_online_status(state, req, true).await
    }

    async fn logoff(state: Arc<Mutex<Self>>, req: Request) -> Result<Response> {
        Self::change_online_status(state, req, false).await
    }

    async fn finger(state: Arc<Mutex<Self>>, req: Request) -> Result<Response> {
        let from_user: JSONResponse = if let Ok(username) = Self::check_key(&state, &req).await {
            let lock = state.lock().await;
            lock.users.get(&username).unwrap().into()
        } else {
            JSONResponse::User {
                username: "anonymous".to_owned(),
                status: JSONStatus::default(),
            }
        };

        let mut lock = state.lock().await;
        if let Some(usern) = req.params.get("user") {
            if let Some(user) = lock.users.get_mut(usern) {
                user.add_log(from_user);
                Ok(Response::from(ResponseStatus::Ok, user))
            } else {
                Ok(Response::from(
                    ResponseStatus::NotFound,
                    JSONResponse::Error("user not found".to_owned()),
                ))
            }
        } else {
            Ok(Response::from(
                ResponseStatus::Bad,
                JSONResponse::Error("a user is required".to_owned()),
            ))
        }
    }

    async fn check(state: Arc<Mutex<Self>>, req: Request) -> Result<Response> {
        let username = Self::check_key(&state, &req).await?;
        let mut lock = state.lock().await;
        let log = lock.users.get_mut(&username).unwrap().log();

        Ok(Response::from(ResponseStatus::Ok, JSONResponse::List(log)))
    }

    async fn bump(state: Arc<Mutex<Self>>, req: Request) -> Result<Response> {
        let username = Self::check_key(&state, &req).await?;
        let mut lock = state.lock().await;
        let user = lock.users.get_mut(&username).unwrap();
        user.bump();

        Ok(Response::from(
            ResponseStatus::Ok,
            JSONResponse::OK("you are bumped".to_owned()),
        ))
    }

    async fn list(state: Arc<Mutex<Self>>, _: Request) -> Result<Response> {
        let mut output: Vec<JSONResponse> = vec![];
        let lock = state.lock().await;
        // let users = lock.users.len()?;

        for (_, user) in lock.users.iter() {
            output.push(user.into())
        }

        Ok(Response::from(
            ResponseStatus::Ok,
            JSONResponse::List(output),
        ))
    }

    async fn register(state: Arc<Mutex<Self>>, req: Request) -> Result<Response> {
        let mut lock = state.lock().await;

        // check server config if registration is allowed
        if !lock.config.registration {
            return Ok(Response::from(
                ResponseStatus::Unauth,
                JSONResponse::Error("registration is not allowed on this server".to_owned()),
            ));
        }

        if let Some(username) = req.params.get("username") {
            // check server config for registration key
            if let Some(auth_key) = &lock.config.auth_key {
                // get the registration key provided by prospective user
                if let Some(key) = req.params.get("key") {
                    if *key != *auth_key {
                        // key is incorrect
                        return Ok(Response::from(
                            ResponseStatus::Unauth,
                            "server registration key is invalid",
                        ));
                    }
                } else {
                    // key is required but not provided in request
                    return Ok(Response::from(
                        ResponseStatus::Unauth,
                        JSONResponse::Error(
                            "registration key is required on this server".to_owned(),
                        ),
                    ));
                }
            }

            let ulpath = lock.config.users_list.clone();
            lock.users
                .register(username.to_owned(), &ulpath, req.params.get("password"))
                .await?;
            // let uid = uuid.to_string();
            Ok(Response::from(
                ResponseStatus::Ok,
                JSONResponse::OK("account created".to_owned()),
            ))
        } else {
            Ok(Response::from(
                ResponseStatus::Bad,
                JSONResponse::Error("a username is required to register".to_owned()),
            ))
        }
    }

    async fn deregister(state: Arc<Mutex<Self>>, req: Request) -> Result<Response> {
        let username = Self::check_key(&state, &req).await?;
        let mut lock = state.lock().await;
        let path = lock.config.users_list.clone();
        lock.users.remove(username, &path).await?;

        Ok(Response::from(
            ResponseStatus::Ok,
            JSONResponse::OK("your account has been removed".to_owned()),
        ))
    }
}

// could make this a trait
impl Server {
    pub async fn init(config: Option<PathBuf>) -> Result<Self> {
        let config = Config::load(config).await?;
        let users = UserList::load(&config.users_list).await?;

        Ok(Self {
            config,
            users,
        })
    }

    async fn offline_worker(state: Arc<Mutex<Self>>) -> ! {
        info!("starting offline worker");
        loop {
            sleep(Duration::from_secs(60)).await;
            info!("checking for dead users");
            let mut lock = state.lock().await;
            lock.users.check_statuses();
        }
    }

    pub async fn run(self) -> Result<()> {
        info!("starting finger server...");
        self.lock().await?;
        let listener = TcpListener::bind(&self.config.socket_path).await?;
        info!("listening on '{}'", &self.config.socket_path);

        // make state of the server thread safe.
        let state = Arc::new(Mutex::new(self));
        // let (tx, _rx) = tokio::sync::mpsc::channel(1);

        let ow_state = state.clone();
        tokio::spawn(Self::offline_worker(ow_state));

        info!("server started.");
        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    info!(?addr, "connection...");
                    let mut stream = BufStream::new(stream);
                    let pstate = state.clone();

                    tokio::spawn(async move {
                        let err = match Request::parse(&mut stream).await {
                            Ok(request) => {
                                Self::run_request(pstate, request)
                                    .await
                                    .write(&mut stream)
                                    .await
                            }
                            Err(e) => {
                                error!("parse error: {}", e);
                                Response::from(
                                    ResponseStatus::Bad,
                                    JSONResponse::Error(format!("failed to parse request: {}", e)),
                                )
                                .write(&mut stream)
                                .await
                            }
                        };

                        if let Err(e) = err {
                            error!("server error {e}");
                        }
                    });
                }
                Err(e) => {
                    error!("{}", e);
                    continue;
                }
            }
        }
    }

    async fn run_request(state: Arc<Mutex<Self>>, req: Request) -> Response {
        let r = match req.action {
            Action::Login => Self::login(state, req).await,
            Action::Logoff => Self::logoff(state, req).await,
            Action::Finger => Self::finger(state, req).await,
            Action::Check => Self::check(state, req).await,
            Action::Bump => Self::bump(state, req).await,
            Action::List => Self::list(state, req).await,
            Action::Register => Self::register(state, req).await,
            Action::Deregister => Self::deregister(state, req).await,
        };

        match r {
            Ok(r) => r,
            Err(e) => Response::from(
                ResponseStatus::ServerError,
                JSONResponse::Error(e.to_string()),
            ),
        }
    }

    async fn change_online_status(
        state: Arc<Mutex<Self>>,
        req: Request,
        status: bool,
    ) -> Result<Response> {
        let username = Self::check_key(&state, &req).await?;

        let mut lock = state.lock().await;

        if let Some(user) = lock.users.get_mut(&username) {
            user.set_status(Status {
                online: status,
                text: req
                    .params
                    .get("status")
                    .map(|s| s.replace("+", " "))
                    // .map(String::to_owned)
                    .or(user.status().text.to_owned()),
                since: Instant::now(),
            });
        } else {
            return Ok(Response::from(
                ResponseStatus::NotFound,
                JSONResponse::Error("user not found".to_owned()),
            ));
        }

        if status {
            Ok(Response::from(
                ResponseStatus::Ok,
                JSONResponse::OK("you are now logged on".to_owned()),
            ))
        } else {
            Ok(Response::from(
                ResponseStatus::Ok,
                JSONResponse::OK("you are now logged off".to_owned()),
            ))
        }
    }

    async fn check_key(state: &Arc<Mutex<Self>>, req: &Request) -> Result<String> {
        let lock = state.lock().await;
        let auth = req
            .auth
            .clone()
            .ok_or(anyhow!("no authentication header"))?;
        let username = req
            .params
            .get("username")
            .clone()
            .ok_or(anyhow!("no username"))?;
        let user = lock.users.get(username).ok_or(anyhow!("user not found"))?;
        if !user.compare_key(auth) {
            return Err(anyhow!("invalid authentication"));
        }

        Ok(username.to_owned())
    }

    async fn login(state: Arc<Mutex<Self>>, req: Request) -> Result<Response> {
        Self::change_online_status(state, req, true).await
    }

    async fn logoff(state: Arc<Mutex<Self>>, req: Request) -> Result<Response> {
        Self::change_online_status(state, req, false).await
    }

    async fn lock(&self) -> Result<File> {
        is_relative("lock", &self.config.lock)?;
        info!("creating lock at {}", self.config.lock.display());
        Ok(OpenOptions::new()
            .write(true)
            .create(true)
            .open(&self.config.lock)
            .await?)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    info!("loading fingr server resources...");
    #[cfg(debug_assertions)]
    let finger = Server::init(Some(PathBuf::from("./finger.config"))).await?;

    #[cfg(not(debug_assertions))]
    let finger = Server::init(None).await?;

    finger.run().await
}
