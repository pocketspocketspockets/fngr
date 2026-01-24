use std::{path::PathBuf, sync::Arc, time::Duration};

pub mod config;
mod networking;
pub mod prelude;
pub mod userlist;

use anyhow::Error;
use config::Config;
use prelude::*;
use tokio::{
    fs::{File, OpenOptions},
    io::BufStream,
    net::TcpListener,
    sync::{Mutex, mpsc::Sender},
    time::{Instant, sleep},
};
use userlist::UserList;

use crate::{
    networking::{JSONResponse, Request, Response},
    userlist::{JSONStatus, Status},
};

// struct holds the state of the server
struct Fingr {
    config: Config,
    #[allow(unused)]
    lock: Option<File>,
    users: UserList,
}

// could make this a trait
impl Fingr {
    pub async fn init(config: Option<PathBuf>) -> Result<Self> {
        let config = Config::load(config).await?;
        let lock = None;
        let users = UserList::load(&config.users_list).await?;

        Ok(Self {
            config,
            lock,
            users,
        })
    }

    async fn offline_worker(state: Arc<Mutex<Self>>, _tx: Sender<Vec<Error>>) -> ! {
        info!("starting offline worker");
        loop {
            sleep(Duration::from_secs(60)).await;
            info!("checking for dead users");
            let mut lock = state.lock().await;
            lock.users.check_statuses();
        }
    }

    // async fn cooldown_worker(list: Arc<Mutex<HashMap<IpAddr, Instant>>>) {
    //     loop {
    //         sleep(Duration::from_secs(1)).await;
    //     }
    // }

    pub async fn run(self) -> Result<()> {
        info!("starting finger server...");
        self.lock().await?;
        let listener = TcpListener::bind(&self.config.socket_path).await?;
        info!("listening on '{}'", &self.config.socket_path);

        // make state of the server thread safe.
        let state = Arc::new(Mutex::new(self));
        let (tx, _rx) = tokio::sync::mpsc::channel(1);

        let ow_state = state.clone();
        tokio::spawn(Self::offline_worker(ow_state, tx));

        info!("server started.");
        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    info!(?addr, "connection...");
                    let mut stream = BufStream::new(stream);
                    let pstate = state.clone();

                    tokio::spawn(async move {
                        let r = match Request::parse(&mut stream).await {
                            Ok(request) => match Self::run_request(pstate, request).await {
                                Ok(response) => response.write(&mut stream).await,
                                Err(e) => {
                                    error!("{}", e);
                                    Response::from(
                                        networking::ResponseStatus::ServerError,
                                        JSONResponse::Error(e.to_string()),
                                    )
                                    .write(&mut stream)
                                    .await
                                }
                            },
                            Err(e) => {
                                error!("{}", e);
                                Response::from(
                                    networking::ResponseStatus::ServerError,
                                    JSONResponse::Error(e.to_string()),
                                )
                                .write(&mut stream)
                                .await
                            }
                        };

                        if let Err(e) = r {
                            error!("{}", e);
                            Response::from(
                                networking::ResponseStatus::ServerError,
                                JSONResponse::Error(e.to_string()),
                            )
                            .write(&mut stream)
                            .await
                            .unwrap();
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

    async fn run_request(state: Arc<Mutex<Self>>, req: Request) -> Result<Response> {
        match req.action {
            networking::Action::Login => Self::login(state, req).await,
            networking::Action::Logoff => Self::logoff(state, req).await,
            networking::Action::Finger => Self::finger(state, req).await,
            networking::Action::Check => Self::check(state, req).await,
            networking::Action::Bump => Self::bump(state, req).await,
            networking::Action::List => Self::list(state, req).await,
            networking::Action::Register => Self::register(state, req).await,
            networking::Action::Deregister => Self::deregister(state, req).await,
        }
    }

    async fn change_online_status(
        state: Arc<Mutex<Self>>,
        req: Request,
        status: bool,
    ) -> Result<Response> {
        let username = match Self::check_key(&state, &req).await {
            Ok(Ok(content)) => content,
            Ok(Err(res)) => return Ok(res),
            Err(e) => return Err(e),
        };

        let mut lock = state.lock().await;

        if let Some(user) = lock.users.get_mut(&username) {
            user.set_status(Status {
                online: status,
                text: req.status.or(user.status().text.to_owned()),
                since: Instant::now(),
            });
        } else {
            return Ok(Response::from(
                networking::ResponseStatus::NotFound,
                networking::JSONResponse::Error("user not found".to_owned()),
            ));
        }

        if status {
            Ok(Response::from(
                networking::ResponseStatus::Ok,
                networking::JSONResponse::OK("you are now logged on".to_owned()),
            ))
        } else {
            Ok(Response::from(
                networking::ResponseStatus::Ok,
                networking::JSONResponse::OK("you are now logged off".to_owned()),
            ))
        }
    }

    async fn check_key(
        state: &Arc<Mutex<Self>>,
        req: &Request,
    ) -> Result<std::result::Result<String, Response>> {
        if let Some(username) = &req.username {
            if let Some(key) = &req.key {
                let lock = state.lock().await;
                if let Some(user) = lock.users.get(username) {
                    if user.compare_key(key.parse()?) {
                        // user.
                        Ok(Ok(username.to_owned()))
                    } else {
                        Ok(Err(Response::from(
                            networking::ResponseStatus::Unauth,
                            JSONResponse::Error("invalid username or key".to_owned()),
                        )))
                    }
                } else {
                    Ok(Err(Response::from(
                        networking::ResponseStatus::NotFound,
                        JSONResponse::Error("unknown username".to_owned()),
                    )))
                }
            } else {
                Ok(Err(Response::from(
                    networking::ResponseStatus::Bad,
                    JSONResponse::Error("missing key".to_owned()),
                )))
            }
        } else {
            Ok(Err(Response::from(
                networking::ResponseStatus::Bad,
                JSONResponse::Error("missing username".to_owned()),
            )))
        }
    }

    async fn login(state: Arc<Mutex<Self>>, req: Request) -> Result<Response> {
        Self::change_online_status(state, req, true).await
    }

    async fn logoff(state: Arc<Mutex<Self>>, req: Request) -> Result<Response> {
        Self::change_online_status(state, req, false).await
    }

    async fn finger(state: Arc<Mutex<Self>>, req: Request) -> Result<Response> {
        let from_user: JSONResponse = if let Ok(Ok(fuser)) = Self::check_key(&state, &req).await {
            let lock = state.lock().await;
            lock.users.get(&fuser).unwrap().into()
        } else {
            JSONResponse::User {
                username: "anonymous".to_owned(),
                status: JSONStatus::default(),
            }
        };

        let mut lock = state.lock().await;
        if let Some(usern) = req.finger_user {
            if let Some(user) = lock.users.get_mut(&usern) {
                user.add_log(from_user);
                Ok(Response::from(networking::ResponseStatus::Ok, user))
            } else {
                Ok(Response::from(
                    networking::ResponseStatus::NotFound,
                    JSONResponse::Error("user not found".to_owned()),
                ))
            }
        } else {
            Ok(Response::from(
                networking::ResponseStatus::Bad,
                JSONResponse::Error("a user is required".to_owned()),
            ))
        }
    }

    async fn check(state: Arc<Mutex<Self>>, req: Request) -> Result<Response> {
        let username = match Self::check_key(&state, &req).await {
            Ok(Ok(content)) => content,
            Ok(Err(res)) => return Ok(res),
            Err(e) => return Err(e),
        };

        let mut lock = state.lock().await;
        let log = lock.users.get_mut(&username).unwrap().log();

        Ok(Response::from(
            networking::ResponseStatus::Ok,
            JSONResponse::List(log),
        ))
    }

    async fn bump(state: Arc<Mutex<Self>>, req: Request) -> Result<Response> {
        let username = match Self::check_key(&state, &req).await {
            Ok(Ok(content)) => content,
            Ok(Err(res)) => return Ok(res),
            Err(e) => return Err(e),
        };

        let mut lock = state.lock().await;
        let user = lock.users.get_mut(&username).unwrap();
        user.bump();

        Ok(Response::from(
            networking::ResponseStatus::Ok,
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
            networking::ResponseStatus::Ok,
            JSONResponse::List(output),
        ))
    }

    async fn register(state: Arc<Mutex<Self>>, req: Request) -> Result<Response> {
        let mut lock = state.lock().await;

        if !lock.config.registration {
            return Ok(Response::from(
                networking::ResponseStatus::Unauth,
                JSONResponse::Error("registration is not allowed on this server".to_owned()),
            ));
        }

        if let Some(username) = req.username {
            let _v = if let Some(auth_key) = &lock.config.auth_key {
                if let Some(key) = req.key {
                    key == *auth_key
                } else {
                    return Ok(Response::from(
                        networking::ResponseStatus::Unauth,
                        JSONResponse::Error("incorrect registration key".to_owned()),
                    ));
                }
            } else {
                true
            };
            let ulpath = lock.config.users_list.clone();
            let key = lock.users.register(username, &ulpath).await?;
            let key = key.to_string();
            Ok(Response::from(
                networking::ResponseStatus::Ok,
                JSONResponse::OK(key),
            ))
        } else {
            Ok(Response::from(
                networking::ResponseStatus::Bad,
                JSONResponse::Error("a username is required to register".to_owned()),
            ))
        }
    }

    async fn deregister(state: Arc<Mutex<Self>>, req: Request) -> Result<Response> {
        let username = match Self::check_key(&state, &req).await {
            Ok(Ok(content)) => content,
            Ok(Err(res)) => return Ok(res),
            Err(e) => return Err(e),
        };

        let mut lock = state.lock().await;
        let path = lock.config.users_list.clone();
        lock.users.remove(username, &path).await?;

        Ok(Response::from(
            networking::ResponseStatus::Ok,
            JSONResponse::OK("your account has been removed".to_owned()),
        ))
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
    let finger = Fingr::init(Some(PathBuf::from("./finger.config"))).await?;

    #[cfg(not(debug_assertions))]
    let finger = Fingr::init(None).await?;

    finger.run().await
}
