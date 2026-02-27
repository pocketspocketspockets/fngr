mod config;

use config::Config;
use snuggle::{Snuggle, prelude::*};
use snuggle::{
    networking::{Action, JSONResponse, Request, Response, ResponseStatus},
    userlist::{Status, UserList},
};
use uuid::Uuid;

use std::collections::HashMap;
use std::{path::PathBuf, sync::Arc, time::Duration};
use tokio::{
    io::BufStream,
    net::TcpListener,
    sync::Mutex,
    time::{Instant, sleep},
};

// struct holds the state of the server
struct Server {
    config: Config,
    users: UserList,
    fedder: HashMap<String, (JSONResponse, Uuid)>,
}

impl Snuggle for Server {
    type SelfLock = Arc<Mutex<Self>>;

    async fn info(state: Arc<Mutex<Self>>, _: Request) -> Result<Response> {
        Ok(Response::from(
            ResponseStatus::Ok,
            JSONResponse::Info {
                name: env!("CARGO_PKG_NAME").to_owned(),
                version: env!("CARGO_PKG_VERSION").to_owned(),
                licesnse: env!("CARGO_PKG_LICENSE").to_owned(),
                contact: env!("CARGO_PKG_AUTHORS").to_owned(),
                users: state.lock().await.users.count(),
            },
        ))
    }

    async fn login(state: Arc<Mutex<Self>>, req: Request) -> Result<Response> {
        Self::change_online_status(state, req, true).await
    }

    async fn logoff(state: Arc<Mutex<Self>>, req: Request) -> Result<Response> {
        Self::change_online_status(state, req, false).await
    }

    async fn snuggle(state: Arc<Mutex<Self>>, req: Request) -> Result<Response> {
        let (initial_username, federated_username) = Self::check_key(&state, &req).await?;
        let mut lock = state.lock().await;
        let our_server = lock.config.server_name.to_owned();
        let from_user: JSONResponse = lock.users.get_mut(&initial_username).unwrap().into();
        let user = req
            .params
            .get("user")
            .ok_or(anyhow!("a user is required"))?
            .to_owned();
        let mut snuggled_user: &str = &user;
        let mut server: &str = &our_server;

        if snuggled_user.contains("@") {
            let mut split = snuggled_user.split("@");
            snuggled_user = split
                .next()
                .ok_or(anyhow!("failed to parse username '{}'", user))?;
            server = split
                .next()
                .ok_or(anyhow!("failed to parse server '{}'", user))?;
        }

        // reassign as to not edit them
        // TODO: remove, this is unnecesarry
        let snuggled_user: &str = snuggled_user;
        let server: &str = server;

        if our_server != server {
            let fingerprint = uuid::Uuid::from_bytes(rand::random());
            lock.fedder
                .insert(snuggled_user.to_owned(), (from_user, fingerprint));
            // drop the lock to allow other threads to pick it up.
            // This is to make sure function `Server::fingerprint` can use it when the other server checks us.
            drop(lock);

            let url = format!(
                "http://{}/fed_snuggle?snuggle={}&by_user={}&base={}",
                server, snuggled_user, federated_username, fingerprint
            );
            info!(?url);
            let r = reqwest::get(url).await?;
            let resp = r.text().await?;
            let j: JSONResponse = serde_json::from_str(&resp)?;
            Ok(Response::from(ResponseStatus::Ok, j))
        } else {
            // let mut lock = state.lock().await;
            let user = lock
                .users
                .get_mut(snuggled_user)
                .ok_or(anyhow!("user '{}' not found", snuggled_user))?;
            user.add_log(from_user);
            Ok(Response::from(ResponseStatus::Ok, user))
        }
    }

    async fn check(state: Arc<Mutex<Self>>, req: Request) -> Result<Response> {
        let (username, _) = Self::check_key(&state, &req).await?;
        let mut lock = state.lock().await;
        let log = lock.users.get_mut(&username).unwrap().log();

        Ok(Response::from(ResponseStatus::Ok, JSONResponse::List(log)))
    }

    async fn bump(state: Arc<Mutex<Self>>, req: Request) -> Result<Response> {
        let (username, _) = Self::check_key(&state, &req).await?;
        let mut lock = state.lock().await;
        let user = lock.users.get_mut(&username).unwrap();
        user.bump();

        Ok(Response::from(
            ResponseStatus::Ok,
            JSONResponse::Ok("you are bumped".to_owned()),
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

        if let Some(username) = req.username {
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
            let server = lock.config.server_name.clone();
            lock.users
                .register(
                    username.to_owned(),
                    &ulpath,
                    req.params.get("password"),
                    server,
                )
                .await?;
            Ok(Response::from(
                ResponseStatus::Ok,
                JSONResponse::Ok("account created".to_owned()),
            ))
        } else {
            Ok(Response::from(
                ResponseStatus::Bad,
                JSONResponse::Error("a username is required to register".to_owned()),
            ))
        }
    }

    async fn deregister(state: Arc<Mutex<Self>>, req: Request) -> Result<Response> {
        let (username, _) = Self::check_key(&state, &req).await?;
        let mut lock = state.lock().await;
        let path = lock.config.users_list.clone();
        lock.users.remove(username, &path).await?;

        Ok(Response::from(
            ResponseStatus::Ok,
            JSONResponse::Ok("your account has been removed".to_owned()),
        ))
    }

    async fn set_bio(state: Self::SelfLock, req: Request) -> Result<Response> {
        let (username, _) = Self::check_key(&state, &req).await?;
        let mut lock = state.lock().await;
        if let Some(user) = lock.users.get_mut(&username) {
            let bio = req.params.get("bio").map(Clone::clone);
            user.set_bio(bio);
            lock.save_users().await?;
        }

        Ok(Response::from(
            ResponseStatus::Ok,
            JSONResponse::Ok("Ok".to_owned()),
        ))
    }

    async fn add_social(state: Self::SelfLock, req: Request) -> Result<Response> {
        let (username, _) = Self::check_key(&state, &req).await?;

        if let (Some(name), Some(info)) = (req.params.get("name"), req.params.get("string")) {
            let mut lock = state.lock().await;
            let user = lock.users.get_mut(&username).unwrap();
            user.add_social(name.to_owned(), info.to_owned());
            lock.save_users().await?;
        }

        Ok(Response::from(
            ResponseStatus::Ok,
            JSONResponse::Ok("Ok".to_owned()),
        ))
    }

    async fn remove_social(state: Self::SelfLock, req: Request) -> Result<Response> {
        let (username, _) = Self::check_key(&state, &req).await?;

        if let Some(name) = req.params.get("name") {
            let mut lock = state.lock().await;
            let user = lock.users.get_mut(&username).unwrap();
            user.remove_social(name.to_owned());
            lock.save_users().await?;
        }

        Ok(Response::from(
            ResponseStatus::Ok,
            JSONResponse::Ok("Ok".to_owned()),
        ))
    }

    async fn set_website(state: Self::SelfLock, req: Request) -> Result<Response> {
        let (username, _) = Self::check_key(&state, &req).await?;

        // if let Some(address) = req.params.get("addr") {
        let mut lock = state.lock().await;
        let user = lock.users.get_mut(&username).unwrap();

        user.set_website(req.params.get("addr").map(|s| s.to_owned()));
        // }

        Ok(Response::from(
            ResponseStatus::Ok,
            JSONResponse::Ok("Ok".to_owned()),
        ))
    }

    /// FEDERATION BYEAAAAHHHHH
    async fn fingerprint(state: Self::SelfLock, req: Request) -> Result<Response> {
        let snuggled_user = req
            .params
            .get("user")
            .ok_or(anyhow!("a user is required"))?;
        let base = req
            .params
            .get("base")
            .map(|s| s.parse::<Uuid>())
            .ok_or(anyhow!("fingerprint `base` required"))??;

        let mut lock = state.lock().await;
        let (from_user, fingerprint) = lock
            .fedder
            .remove(snuggled_user)
            .ok_or(anyhow!("entry not found"))?;
        drop(lock);

        if base == fingerprint {
            Ok(Response::from(ResponseStatus::Ok, from_user))
        } else {
            Ok(Response::from(
                ResponseStatus::Bad,
                JSONResponse::Error("incorrect fingerprint".to_owned()),
            ))
        }
    }

    async fn fed_snuggle(state: Self::SelfLock, req: Request) -> Result<Response> {
        let snuggled_user = req
            .params
            .get("snuggle")
            .ok_or(anyhow!("snuggle is requied"))?;
        let initiating_f_user = req
            .params
            .get("by_user")
            .ok_or(anyhow!("`by_user` is required"))?;
        let fingerprint = req
            .params
            .get("base")
            .ok_or(anyhow!("`base` fingerprint is required"))?;

        let mut split = initiating_f_user.split("@");
        let _ = split
            .next()
            .ok_or(anyhow!("error parsing username '{}'", initiating_f_user))?;
        let initial_server = split
            .next()
            .ok_or(anyhow!("error parsing server '{}'", initiating_f_user))?;

        let url = format!(
            "http://{}/fingerprint?user={}&base={}",
            initial_server, snuggled_user, fingerprint
        );
        info!(?url);
        let resp = reqwest::get(url).await?;
        let text = resp.text().await?;
        let from_user: JSONResponse = serde_json::from_str(&text)?;

        match from_user {
            JSONResponse::User { .. } => {
                let mut lock = state.lock().await;
                let snuggled_user = lock
                    .users
                    .get_mut(snuggled_user)
                    .ok_or(anyhow!("user {} not found", snuggled_user))?;
                snuggled_user.add_log(from_user);
                Ok(Response::from(ResponseStatus::Ok, snuggled_user))
            }

            _ => Ok(Response::from(ResponseStatus::Bad, from_user)),
        }
    }
}

// could make this a trait
impl Server {
    async fn save_users(&mut self) -> Result<()> {
        let p = &self.config.users_list;
        self.users.save(&p).await?;

        Ok(())
    }

    pub async fn init(config: Option<PathBuf>) -> Result<Self> {
        let config = Config::load(config).await?;
        let users = UserList::load(&config.server_name, &config.users_list).await?;

        Ok(Self {
            config,
            users,
            fedder: HashMap::new(),
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
        info!("starting snuggle server...");
        let listener = TcpListener::bind(&self.config.socket_path).await?;
        info!("listening on '{}'", &self.config.socket_path);

        // make state of the server thread safe.
        let state = Arc::new(Mutex::new(self));

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
                        let err = match Request::parse_buf(&mut stream).await {
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
        println!("{:#?}", req);
        let r = match req.action {
            Action::Login => Self::login(state, req).await,
            Action::Logoff => Self::logoff(state, req).await,
            Action::Snuggle => Self::snuggle(state, req).await,
            Action::Check => Self::check(state, req).await,
            Action::Bump => Self::bump(state, req).await,
            Action::List => Self::list(state, req).await,
            Action::Register => Self::register(state, req).await,
            Action::Deregister => Self::deregister(state, req).await,
            Action::Info => Self::info(state, req).await,
            Action::SetBio => Self::set_bio(state, req).await,
            Action::AddSocial => Self::add_social(state, req).await,
            Action::DelSocial => Self::remove_social(state, req).await,
            Action::SetWeb => Self::set_website(state, req).await,
            Action::Fingerprint => Self::fingerprint(state, req).await,
            Action::FedSnuggle => Self::fed_snuggle(state, req).await,
        };

        match r {
            Ok(r) => r,
            Err(e) => {
                warn!("{}", e);
                let e = Response::from(
                    ResponseStatus::ServerError,
                    JSONResponse::Error(e.to_string()),
                );
                e
            }
        }
    }

    async fn change_online_status(
        state: Arc<Mutex<Self>>,
        req: Request,
        status: bool,
    ) -> Result<Response> {
        let (username, _) = Self::check_key(&state, &req).await?;

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
                JSONResponse::Ok("you are now logged on".to_owned()),
            ))
        } else {
            Ok(Response::from(
                ResponseStatus::Ok,
                JSONResponse::Ok("you are now logged off".to_owned()),
            ))
        }
    }

    async fn check_key(state: &Arc<Mutex<Self>>, req: &Request) -> Result<(String, String)> {
        let lock = state.lock().await;
        let auth = req
            .authorization
            .clone()
            .ok_or(anyhow!("no authorization header"))?;
        let username = req.username.clone().ok_or(anyhow!("no username"))?;
        let user = lock.users.get(&username).ok_or(anyhow!("user not found"))?;
        if !user.compare_key(auth) {
            return Err(anyhow!("invalid authorization"));
        }

        Ok((username, user.username()))
    }

    async fn login(state: Arc<Mutex<Self>>, req: Request) -> Result<Response> {
        Self::change_online_status(state, req, true).await
    }

    async fn logoff(state: Arc<Mutex<Self>>, req: Request) -> Result<Response> {
        Self::change_online_status(state, req, false).await
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    info!("loading fingr server resources...");
    #[cfg(debug_assertions)]
    let snuggle = Server::init(Some(PathBuf::from("./snuggle.config"))).await?;

    #[cfg(not(debug_assertions))]
    let snuggle = Server::init(None).await?;

    snuggle.run().await
}
