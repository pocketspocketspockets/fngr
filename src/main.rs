#![forbid(clippy::unwrap_used)]
#![forbid(clippy::expect_used)]
#![forbid(unsafe_code)]

use crate::user::Username;
use crate::{authorization::Authorization, config::Config, database::Database, user::User};
use lazy_static::lazy_static;
use rocket::{catch,catchers};
use rocket::{
    get,
    http,
    post, routes,
    serde::uuid::Uuid,
};
use sha_rs::Sha;
use time::UtcDateTime;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::Mutex;
use tracing::{error, warn};

mod authorization;
mod config;
mod database;
mod jobject;
mod prelude;
mod user;
// mod error;

type Result = (http::Status, String);

#[cfg(debug_assertions)]
lazy_static! {
    /// Stores config globally. If function needs both config and database, lock database first.
    static ref CONFIG: Mutex<Config> = Mutex::new(Config::load(Some("./snuggle.config")).unwrap());
}

#[cfg(not(debug_assertions))]
lazy_static! {
    static ref CONFIG: Mutex<Config> = Mutex::new(Config::load(None).unwrap());
}

lazy_static! {
    static ref DATABASE: Mutex<Database> = Mutex::new(
        Database::load(&CONFIG.lock().unwrap().database).unwrap()
    );
}

#[get("/info")]
fn info() -> Result {
    let count = DATABASE.lock().unwrap().count();

    let count = match count {
        Ok(c) => c,
        Err(e) => {
            error!("{}", e);
            return (
                http::Status::new(500),
                jobject::Error(e.to_string()).to_string(),
            );
        }
    };

    let info = jobject::Info {
        name: env!("CARGO_PKG_NAME"),
        version: env!("CARGO_PKG_VERSION"),
        api_version: "1.0",
        license: env!("CARGO_PKG_LICENSE"),
        contact: env!("CARGO_PKG_AUTHORS"),
        users: count,
    };

    let j = serde_json::to_string(&info);

    match j {
        Ok(j) => (http::Status::new(200), j),
        Err(e) => {
            error!("{}", e);
            (
                http::Status::new(500),
                jobject::Error(e.to_string()).to_string(),
            )
        }
    }
}

fn auth_filter(auth: Option<&str>, hauth: Option<Authorization>) -> anyhow::Result<Authorization> {
    match hauth {
        Some(h) => Ok(h),
        None => match auth {
            Some(h) => Ok(Authorization::from_str(h)),
            None => Err(anyhow::anyhow!("authorization required")),
        },
    }
}

#[get("/login?<username>&<auth>&<status>")]
async fn login(
    username: &str,
    status: Option<&str>,
    auth: Option<&str>,
    hauth: Option<Authorization>,
) -> Result {
    let auth = match auth_filter(auth, hauth) {
        Ok(a) => a,
        Err(e) => {
            error!("{}", e);
            return (http::Status::Unauthorized, jobject::Error(e.to_string()).to_string())
        }
    };

    let username: Username = match username.parse() {
        Ok(u) => u,
        Err(e) => {
            error!("{}", e);
            return (http::Status::new(500), jobject::Error(e.to_string()).to_string())
        }
    };

    let mut db = DATABASE.lock().unwrap();
    if let Err(e) = db.authorize(&username, auth.to_string()) {
        error!("{}", e);
        return (http::Status::new(401), jobject::Error(e.to_string()).to_string())
    }

    if let Err(e) = db.set_user_status_state(&username, true, UtcDateTime::now()) {
        error!("{}", e);
        return (http::Status::new(500), jobject::Error(e.to_string()).to_string())
    }

    if let Some(status) = status {
        if let Err(e) = db.set_user_status_message(&username, status) {
            error!("{}", e);
            return (http::Status::new(500), jobject::Error(e.to_string()).to_string())
        }
    }
    
    (http::Status::new(200), "{ \"Ok\": \"logged in\" }".to_owned())
}

#[get("/logoff?<username>&<auth>")]
async fn logoff(username: &str, auth: Option<&str>, hauth: Option<Authorization>) -> Result {
    let auth = match auth_filter(auth, hauth) {
        Ok(a) => a,
        Err(e) => {
            error!("{}", e);
            return (http::Status::Unauthorized, jobject::Error(e.to_string()).to_string())
        }
    };

    let username: Username = match username.parse() {
        Ok(u) => u,
        Err(e) => {
            error!("{}", e);
            return (http::Status::new(500), jobject::Error(e.to_string()).to_string())
        }
    };

    let mut db = DATABASE.lock().unwrap();
    if let Err(e) = db.authorize(&username, auth.to_string()) {
        error!("{}", e);
        return (http::Status::new(401), jobject::Error(e.to_string()).to_string())
    }

    if let Err(e) = db.set_user_status_state(&username, false, UtcDateTime::now()) {
        error!("{}", e);
        return (http::Status::new(500), jobject::Error(e.to_string()).to_string())
    }
    
    (http::Status::new(200), "{ \"Ok\": \"logged in\" }".to_owned())
}

#[get("/snuggle?<username>&<user>&<auth>")]
async fn snuggle(
    username: &str,
    user: &str,
    auth: Option<&str>,
    hauth: Option<Authorization>,
) -> Result {
    let auth = match auth_filter(auth, hauth) {
        Ok(a) => a,
        Err(e) => {
            error!("{}", e);
            return (http::Status::Unauthorized, jobject::Error(e.to_string()).to_string())
        }
    };

    let by_user: Username = match username.parse() {
        Ok(u) => u,
        Err(e) => {
            error!("{}", e);
            return (http::Status::new(500), jobject::Error(e.to_string()).to_string())
        }
    };

    let to_user: Username = match user.parse() {
        Ok(u) => u,
        Err(e) => {
            error!("{}", e);
            return (http::Status::new(500), jobject::Error(e.to_string()).to_string())
        }
    };

    let mut db = DATABASE.lock().unwrap();
    if let Err(e) = db.authorize(&by_user, auth.to_string()) {
        error!("{}", e);
        return (http::Status::new(401), jobject::Error(e.to_string()).to_string())
    }

    if let Err(e) = db.add_to_log(&to_user, &by_user) {
        error!("{}", e);
        return (http::Status::new(500), jobject::Error(e.to_string()).to_string())
    }

    let user: jobject::User = db.get_user(&to_user).unwrap().into();
    let user = serde_json::to_string(&user).unwrap();

    (http::Status::new(200), user)
}

#[get("/check?<username>&<auth>")]
async fn check(username: &str, auth: Option<&str>, hauth: Option<Authorization>) -> Result {
    let auth = match auth_filter(auth, hauth) {
        Ok(a) => a,
        Err(e) => {
            error!("{}", e);
            return (http::Status::Unauthorized, jobject::Error(e.to_string()).to_string())
        }
    };

    let username: Username = match username.parse() {
        Ok(u) => u,
        Err(e) => {
            error!("{}", e);
            return (http::Status::new(500), jobject::Error(e.to_string()).to_string())
        }
    };

    let mut db = DATABASE.lock().unwrap();
    if let Err(e) = db.authorize(&username, auth.to_string()) {
        error!("{}", e);
        return (http::Status::new(401), jobject::Error(e.to_string()).to_string())
    }

    let log = match db.reset_log(&username) {
        Ok(l) => l,
        Err(e) => {
            error!("{}", e);
            return (http::Status::new(500), jobject::Error(e.to_string()).to_string())
        }
    };

    let log = match serde_json::to_string(&log) {
        Ok(l) => l,
        Err(e) => {
            error!("{}", e);
            return (http::Status::new(500), jobject::Error(e.to_string()).to_string())
        }
    };

    (http::Status::new(200), log)
}

#[get("/bump?<username>&<auth>")]
async fn bump(username: &str, auth: Option<&str>, hauth: Option<Authorization>) -> Result {
    let auth = match auth_filter(auth, hauth) {
        Ok(a) => a,
        Err(e) => {
            error!("{}", e);
            return (http::Status::Unauthorized, jobject::Error(e.to_string()).to_string())
        }
    };

    let username: Username = match username.parse() {
        Ok(u) => u,
        Err(e) => {
            error!("{}", e);
            return (http::Status::new(500), jobject::Error(e.to_string()).to_string())
        }
    };

    let mut db = DATABASE.lock().unwrap();
    if let Err(e) = db.authorize(&username, auth.to_string()) {
        error!("{}", e);
        return (http::Status::new(401), jobject::Error(e.to_string()).to_string())
    }

    match db.set_user_status_bump(&username, Some(UtcDateTime::now())) {
        Ok(()) => (http::Status::new(200), "{ \"Ok\": \"You are bumped\" }".to_owned()),
        Err(e) => {
            error!("{}", e);
            (http::Status::new(500), jobject::Error(e.to_string()).to_string())
        }
    }
}

#[get("/list")]
async fn list() -> Result {
    let mut db = DATABASE.lock().unwrap();

    let users = match db.get_internal_users() {
        Ok(us) => us,
        Err(e) => {
            error!("{}", e);
            return (
                http::Status::new(500),
                jobject::Error(e.to_string()).to_string(),
            );
        }
    };

    let mut userslistusers = Vec::new();

    for user in users {
        userslistusers.push(jobject::UserListUser {
            username: user.username().to_string(),
            status: user.status().to_owned().into(),
        })
    }

    let j = match serde_json::to_string(&userslistusers) {
        Ok(j) => j,
        Err(e) => {
            error!("{}", e);
            return (
                http::Status::new(500),
                jobject::Error(e.to_string()).to_string(),
            );
        }
    };

    (http::Status::new(200), j)
}

#[get("/register?<username>&<password>")]
async fn register(username: &str, password: &str) -> Result {
    let mut db = DATABASE.lock().unwrap();
    let config = CONFIG.lock().unwrap();

    let username_to_parse = if username.contains("@") {
        username.to_owned()
    } else {
        format!("{}@{}", username, config.server_name)
    };

    let username = username_to_parse.parse().unwrap();

    match db.get_user(&username) {
        Ok(_) => {
            return (
                http::Status::new(403),
                jobject::Error("user already exists".to_owned()).to_string(),
            );
        }
        Err(e) => warn!("{}: {}", "unhandled error", e),
    };

    let sha = sha_rs::Sha256::new();
    let hash = sha.digest(password.as_bytes());

    let user = User::new(
        username,
        Some(hash),
        Default::default(),
        Default::default(),
        Default::default(),
        Default::default(),
        Default::default(),
    );

    if let Err(e) = db.add_user(user) {
        error!("{}", e);
        (
            http::Status::new(500),
            jobject::Error(e.to_string()).to_string(),
        )
    } else {
        (
            http::Status::new(201),
            "{ \"Ok\": \"User created\" }".to_owned(),
        )
    }
}

#[get("/deregister?<username>&<auth>")]
async fn deregister(username: &str, auth: Option<&str>, hauth: Option<Authorization>) -> Result {
    let auth = match auth_filter(auth, hauth) {
        Ok(a) => a,
        Err(e) => {
            error!("{}", e);
            return (http::Status::Unauthorized, jobject::Error(e.to_string()).to_string())
        }
    };

    let username: Username = match username.parse() {
        Ok(u) => u,
        Err(e) => {
            error!("{}", e);
            return (http::Status::new(500), jobject::Error(e.to_string()).to_string())
        }
    };

    let mut db = DATABASE.lock().unwrap();
    if let Err(e) = db.authorize(&username, auth.to_string()) {
        error!("{}", e);
        return (http::Status::new(401), jobject::Error(e.to_string()).to_string())
    }

    match db.delete_user(&username) {
        Ok(_) => (http::Status::new(200), "\"Ok\": \"you are deregistered\" }".to_owned()),
        Err(e) => {
            error!("{}", e);
            (http::Status::new(401), jobject::Error(e.to_string()).to_string())
        }
    }
}

#[get("/setbio?<username>&<auth>&<bio>")]
async fn set_bio(
    username: &str,
    auth: Option<&str>,
    bio: Option<&str>,
    hauth: Option<Authorization>,
) -> Result {
    // let mut db = DATABASE.lock().unwrap();
    // let username = username.parse().unwrap();
    // db.set_user_bio(&username, bio).unwrap();
    // (http::Status::new(200), "{ \"Ok\": \"status set\" }".to_owned())
    unimplemented!()
}

#[get("/addsocial?<username>&<auth>&<name>&<string>")]
async fn add_social(
    username: &str,
    auth: Option<&str>,
    name: &str,
    string: &str,
    hauth: Option<Authorization>,
) -> Result {
    unimplemented!()
}

#[get("/delsocial?<username>&<auth>&<name>")]
async fn del_social(
    username: &str,
    auth: Option<&str>,
    name: &str,
    hauth: Option<Authorization>,
) -> Result {
    unimplemented!()
}

#[get("/setweb?<username>&<auth>&<addr>")]
async fn website(
    username: &str,
    auth: Option<&str>,
    addr: Option<&str>,
    hauth: Option<Authorization>,
) -> Result {
    unimplemented!()
}

#[post("/<fingerprint>/<snuggled>/<from>")]
async fn fed_snuggle(fingerprint: Uuid, snuggled: &str, from: &str) -> Result {
    unimplemented!()
}

#[get("/fingerprint/<user>/<fingerprint>")]
async fn fingerprint(user: &str, fingerprint: Uuid) -> Result {
    unimplemented!()
}

#[catch(500)]
async fn e500() -> String {
    jobject::Error("500".to_owned()).to_string()
}

#[catch(default)]
async fn edefault() -> String {
    jobject::Error("error".to_owned()).to_string()
}

#[rocket::main]
async fn main() {
    let address = CONFIG
        .lock()
        .unwrap()
        .address
        .clone()
        .map(|a| a.parse().unwrap())
        .unwrap_or(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
    let port = CONFIG.lock().unwrap().port.unwrap_or(5377);

    let rconfig = rocket::Config {
        address,
        port,
        ..Default::default()
    };

    rocket::build()
        .mount(
            "/",
            routes![
                info, login, logoff, snuggle, check, bump, list, register, deregister, set_bio,
                add_social, del_social, website,
            ],
        )
        .mount("/fed", routes![fed_snuggle, fingerprint])
        .register("/", catchers![e500, edefault])
        .register("/fed", catchers![e500, edefault])
        .configure(rconfig)
        .launch()
        .await
        .unwrap();
}
