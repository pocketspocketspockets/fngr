#![forbid(clippy::unwrap_used)]
#![forbid(clippy::expect_used)]
#![forbid(unsafe_code)]

use crate::jobject::SnuggleLog;
use crate::user::Username;
use crate::{authorization::Authorization, config::Config, database::Database, user::User};
use lazy_static::lazy_static;
use rocket::{catch, catchers};
use rocket::{get, http, post, routes, serde::uuid::Uuid};
use sha_rs::Sha;
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::Mutex;
use time::UtcDateTime;
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
    static ref DATABASE: Mutex<Database> =
        Mutex::new(Database::load(&CONFIG.lock().unwrap().database).unwrap());
    static ref FEDERATION: Mutex<HashMap<String, Uuid>> = Mutex::new(HashMap::new());
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
            return (
                http::Status::Unauthorized,
                jobject::Error(e.to_string()).to_string(),
            );
        }
    };

    let username: Username = match username.parse() {
        Ok(u) => u,
        Err(e) => {
            error!("{}", e);
            return (
                http::Status::new(500),
                jobject::Error(e.to_string()).to_string(),
            );
        }
    };

    let mut db = DATABASE.lock().unwrap();
    if let Err(e) = db.authorize(&username, auth.to_string()) {
        error!("{}", e);
        return (
            http::Status::new(401),
            jobject::Error(e.to_string()).to_string(),
        );
    }

    if let Err(e) = db.set_user_status_state(&username, true, UtcDateTime::now()) {
        error!("{}", e);
        return (
            http::Status::new(500),
            jobject::Error(e.to_string()).to_string(),
        );
    }

    if let Some(status) = status {
        if let Err(e) = db.set_user_status_message(&username, status) {
            error!("{}", e);
            return (
                http::Status::new(500),
                jobject::Error(e.to_string()).to_string(),
            );
        }
    }

    (
        http::Status::new(200),
        "{ \"Ok\": \"logged in\" }".to_owned(),
    )
}

#[get("/logoff?<username>&<auth>")]
async fn logoff(username: &str, auth: Option<&str>, hauth: Option<Authorization>) -> Result {
    let auth = match auth_filter(auth, hauth) {
        Ok(a) => a,
        Err(e) => {
            error!("{}", e);
            return (
                http::Status::Unauthorized,
                jobject::Error(e.to_string()).to_string(),
            );
        }
    };

    let username: Username = match username.parse() {
        Ok(u) => u,
        Err(e) => {
            error!("{}", e);
            return (
                http::Status::new(500),
                jobject::Error(e.to_string()).to_string(),
            );
        }
    };

    let mut db = DATABASE.lock().unwrap();
    if let Err(e) = db.authorize(&username, auth.to_string()) {
        error!("{}", e);
        return (
            http::Status::new(401),
            jobject::Error(e.to_string()).to_string(),
        );
    }

    if let Err(e) = db.set_user_status_state(&username, false, UtcDateTime::now()) {
        error!("{}", e);
        return (
            http::Status::new(500),
            jobject::Error(e.to_string()).to_string(),
        );
    }

    (
        http::Status::new(200),
        "{ \"Ok\": \"logged in\" }".to_owned(),
    )
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
            return (
                http::Status::Unauthorized,
                jobject::Error(e.to_string()).to_string(),
            );
        }
    };

    let username = if !username.contains("@") {
        format!("{}@{}", username, CONFIG.lock().unwrap().server_name)
    } else {
        username.to_owned()
    };

    let user = if !user.contains("@") {
        format!("{}@{}", user, CONFIG.lock().unwrap().server_name)
    } else {
        user.to_owned()
    };

    let by_user: Username = match username.parse() {
        Ok(u) => u,
        Err(e) => {
            error!("{}", e);
            return (
                http::Status::new(500),
                jobject::Error(e.to_string()).to_string(),
            );
        }
    };

    let to_user: Username = match user.parse() {
        Ok(u) => u,
        Err(e) => {
            error!("{}", e);
            return (
                http::Status::new(500),
                jobject::Error(e.to_string()).to_string(),
            );
        }
    };

    let user: jobject::User = if to_user.server() != CONFIG.lock().unwrap().server_name {
        // federated
        let uuid = Uuid::from_bytes(rand::random());

        FEDERATION.lock().unwrap().insert(by_user.to_string(), uuid);

        let resp = match reqwest::get(format!(
            "https://{}/{}/{}/{}",
            to_user.server(),
            uuid,
            to_user,
            by_user
        ))
        .await
        {
            Ok(r) => r,
            Err(e) => {
                error!("{}", e);
                return (
                    http::Status::new(500),
                    jobject::Error(e.to_string()).to_string(),
                );
            }
        };

        let resp = match resp.text().await {
            Ok(a) => a,
            Err(e) => {
                error!("{}", e);
                return (
                    http::Status::new(500),
                    jobject::Error(e.to_string()).to_string(),
                );
            }
        };

        match serde_json::from_str(&resp) {
            Ok(u) => u,
            Err(e) => {
                error!("{}", e);
                return (
                    http::Status::new(500),
                    jobject::Error(e.to_string()).to_string(),
                );
            }
        }
    } else {
        // non federated
        let mut db = DATABASE.lock().unwrap();
        if let Err(e) = db.authorize(&by_user, auth.to_string()) {
            error!("{}", e);
            return (
                http::Status::new(401),
                jobject::Error(e.to_string()).to_string(),
            );
        }

        if let Err(e) = db.add_to_log(&to_user, &by_user) {
            error!("{}", e);
            return (
                http::Status::new(500),
                jobject::Error(e.to_string()).to_string(),
            );
        }

        match db.get_user(&to_user) {
            Ok(u) => u.into(),
            Err(e) => {
                error!("{}", e);
                return (
                    http::Status::new(500),
                    jobject::Error(e.to_string()).to_string(),
                );
            }
        }
    };

    match serde_json::to_string(&user) {
        Ok(user) => (http::Status::new(200), user),
        Err(e) => {
            error!("{}", e);
            (
                http::Status::new(500),
                jobject::Error(e.to_string()).to_string(),
            )
        }
    }
}

#[get("/check?<username>&<auth>")]
async fn check(username: &str, auth: Option<&str>, hauth: Option<Authorization>) -> Result {
    let auth = match auth_filter(auth, hauth) {
        Ok(a) => a,
        Err(e) => {
            error!("{}", e);
            return (
                http::Status::Unauthorized,
                jobject::Error(e.to_string()).to_string(),
            );
        }
    };

    let username: Username = match username.parse() {
        Ok(u) => u,
        Err(e) => {
            error!("{}", e);
            return (
                http::Status::new(500),
                jobject::Error(e.to_string()).to_string(),
            );
        }
    };

    let mut db = DATABASE.lock().unwrap();
    if let Err(e) = db.authorize(&username, auth.to_string()) {
        error!("{}", e);
        return (
            http::Status::new(401),
            jobject::Error(e.to_string()).to_string(),
        );
    }

    let log = match db.reset_log(&username) {
        Ok(l) => l,
        Err(e) => {
            error!("{}", e);
            return (
                http::Status::new(500),
                jobject::Error(e.to_string()).to_string(),
            );
        }
    };

    let log = match serde_json::to_string(&log) {
        Ok(l) => l,
        Err(e) => {
            error!("{}", e);
            return (
                http::Status::new(500),
                jobject::Error(e.to_string()).to_string(),
            );
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
            return (
                http::Status::Unauthorized,
                jobject::Error(e.to_string()).to_string(),
            );
        }
    };

    let username: Username = match username.parse() {
        Ok(u) => u,
        Err(e) => {
            error!("{}", e);
            return (
                http::Status::new(500),
                jobject::Error(e.to_string()).to_string(),
            );
        }
    };

    let mut db = DATABASE.lock().unwrap();
    if let Err(e) = db.authorize(&username, auth.to_string()) {
        error!("{}", e);
        return (
            http::Status::new(401),
            jobject::Error(e.to_string()).to_string(),
        );
    }

    match db.set_user_status_bump(&username, Some(UtcDateTime::now())) {
        Ok(()) => (
            http::Status::new(200),
            "{ \"Ok\": \"You are bumped\" }".to_owned(),
        ),
        Err(e) => {
            error!("{}", e);
            (
                http::Status::new(500),
                jobject::Error(e.to_string()).to_string(),
            )
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

#[get("/register?<username>&<password>&<key>")]
async fn register(username: &str, password: &str, key: Option<&str>) -> Result {
    let mut db = DATABASE.lock().unwrap();
    let config = CONFIG.lock().unwrap();

    if !config.registration {
        return (http::Status::Forbidden, jobject::Error("registration is disabled".to_owned()).to_string());
    }

    if let Some(key) = key {
        if let Some(ck) = &config.auth_key {
            if key != ck {
                return (http::Status::Forbidden, jobject::Error("invalid registration key".to_owned()).to_string());
            }
        }
    }

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
            return (
                http::Status::Unauthorized,
                jobject::Error(e.to_string()).to_string(),
            );
        }
    };

    let username: Username = match username.parse() {
        Ok(u) => u,
        Err(e) => {
            error!("{}", e);
            return (
                http::Status::new(500),
                jobject::Error(e.to_string()).to_string(),
            );
        }
    };

    let mut db = DATABASE.lock().unwrap();
    if let Err(e) = db.authorize(&username, auth.to_string()) {
        error!("{}", e);
        return (
            http::Status::new(401),
            jobject::Error(e.to_string()).to_string(),
        );
    }

    match db.delete_user(&username) {
        Ok(_) => (
            http::Status::new(200),
            "\"Ok\": \"you are deregistered\" }".to_owned(),
        ),
        Err(e) => {
            error!("{}", e);
            (
                http::Status::new(401),
                jobject::Error(e.to_string()).to_string(),
            )
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
    let auth = match auth_filter(auth, hauth) {
        Ok(a) => a,
        Err(e) => {
            error!("{}", e);
            return (
                http::Status::Unauthorized,
                jobject::Error(e.to_string()).to_string(),
            );
        }
    };

    let username: Username = match username.parse() {
        Ok(u) => u,
        Err(e) => {
            error!("{}", e);
            return (
                http::Status::new(500),
                jobject::Error(e.to_string()).to_string(),
            );
        }
    };

    let mut db = DATABASE.lock().unwrap();
    if let Err(e) = db.authorize(&username, auth.to_string()) {
        error!("{}", e);
        return (
            http::Status::new(401),
            jobject::Error(e.to_string()).to_string(),
        );
    }

    match db.set_user_bio(&username, bio) {
        Ok(_) => (
            http::Status::new(200),
            "{ \"Ok\": \"Your bio is set\" }".to_owned(),
        ),
        Err(e) => {
            error!("{}", e);
            (
                http::Status::new(500),
                jobject::Error(e.to_string()).to_string(),
            )
        }
    }
}

#[get("/addsocial?<username>&<auth>&<name>&<string>")]
async fn add_social(
    username: &str,
    auth: Option<&str>,
    name: &str,
    string: &str,
    hauth: Option<Authorization>,
) -> Result {
    let auth = match auth_filter(auth, hauth) {
        Ok(a) => a,
        Err(e) => {
            error!("{}", e);
            return (
                http::Status::Unauthorized,
                jobject::Error(e.to_string()).to_string(),
            );
        }
    };

    let username: Username = match username.parse() {
        Ok(u) => u,
        Err(e) => {
            error!("{}", e);
            return (
                http::Status::new(500),
                jobject::Error(e.to_string()).to_string(),
            );
        }
    };

    let mut db = DATABASE.lock().unwrap();
    if let Err(e) = db.authorize(&username, auth.to_string()) {
        error!("{}", e);
        return (
            http::Status::new(401),
            jobject::Error(e.to_string()).to_string(),
        );
    }

    match db.add_user_social(&username, (name, string)) {
        Ok(_) => (
            http::Status::new(200),
            "{ \"Ok\": \"Your social is added\" }".to_owned(),
        ),
        Err(e) => {
            error!("{}", e);
            (
                http::Status::new(500),
                jobject::Error(e.to_string()).to_string(),
            )
        }
    }
}

#[get("/delsocial?<username>&<auth>&<name>")]
async fn del_social(
    username: &str,
    auth: Option<&str>,
    name: &str,
    hauth: Option<Authorization>,
) -> Result {
    let auth = match auth_filter(auth, hauth) {
        Ok(a) => a,
        Err(e) => {
            error!("{}", e);
            return (
                http::Status::Unauthorized,
                jobject::Error(e.to_string()).to_string(),
            );
        }
    };

    let username: Username = match username.parse() {
        Ok(u) => u,
        Err(e) => {
            error!("{}", e);
            return (
                http::Status::new(500),
                jobject::Error(e.to_string()).to_string(),
            );
        }
    };

    let mut db = DATABASE.lock().unwrap();
    if let Err(e) = db.authorize(&username, auth.to_string()) {
        error!("{}", e);
        return (
            http::Status::new(401),
            jobject::Error(e.to_string()).to_string(),
        );
    }

    match db.remove_user_social(&username, name) {
        Ok(_) => (
            http::Status::new(200),
            "{ \"Ok\": \"Your social is removed\" }".to_owned(),
        ),
        Err(e) => {
            error!("{}", e);
            (
                http::Status::new(500),
                jobject::Error(e.to_string()).to_string(),
            )
        }
    }
}

#[get("/setweb?<username>&<auth>&<addr>")]
async fn website(
    username: &str,
    auth: Option<&str>,
    addr: Option<&str>,
    hauth: Option<Authorization>,
) -> Result {
    let auth = match auth_filter(auth, hauth) {
        Ok(a) => a,
        Err(e) => {
            error!("{}", e);
            return (
                http::Status::Unauthorized,
                jobject::Error(e.to_string()).to_string(),
            );
        }
    };

    let username: Username = match username.parse() {
        Ok(u) => u,
        Err(e) => {
            error!("{}", e);
            return (
                http::Status::new(500),
                jobject::Error(e.to_string()).to_string(),
            );
        }
    };

    let mut db = DATABASE.lock().unwrap();
    if let Err(e) = db.authorize(&username, auth.to_string()) {
        error!("{}", e);
        return (
            http::Status::new(401),
            jobject::Error(e.to_string()).to_string(),
        );
    }

    match db.set_user_website(&username, addr) {
        Ok(_) => (
            http::Status::new(200),
            "{ \"Ok\": \"Your website is set\" }".to_owned(),
        ),
        Err(e) => {
            error!("{}", e);
            (
                http::Status::new(500),
                jobject::Error(e.to_string()).to_string(),
            )
        }
    }
}

#[get("/<fingerprint>/<snuggled>/<from>")]
async fn fed_snuggle(fingerprint: Uuid, snuggled: &str, from: &str) -> Result {
    let snuggled: Username = match snuggled.parse() {
        Ok(u) => u,
        Err(e) => {
            error!("{}", e);
            return (
                http::Status::new(500),
                jobject::Error(e.to_string()).to_string(),
            );
        }
    };

    let from: Username = match from.parse() {
        Ok(u) => u,
        Err(e) => {
            error!("{}", e);
            return (
                http::Status::new(500),
                jobject::Error(e.to_string()).to_string(),
            );
        }
    };

    let r = match reqwest::get(format!(
        "https://{}/fed/fingerprint/{}/{}",
        from.server(),
        from,
        fingerprint
    ))
    .await
    {
        Ok(r) => r,
        Err(e) => {
            error!("{}", e);
            return (
                http::Status::new(500),
                jobject::Error(e.to_string()).to_string(),
            );
        }
    };

    let user = match r.text().await {
        Ok(u) => u,
        Err(e) => {
            error!("{}", e);
            return (
                http::Status::new(500),
                jobject::Error(e.to_string()).to_string(),
            );
        }
    };

    let user: jobject::User = match serde_json::from_str(&user) {
        Ok(u) => u,
        Err(e) => {
            error!("{}", e);
            return (
                http::Status::new(500),
                jobject::Error(e.to_string()).to_string(),
            );
        }
    };

    let user: User = User::new(
        user.username.parse().unwrap(),
        None,
        user.status.into(),
        SnuggleLog::default(),
        user.website,
        user.social,
        user.bio,
    );

    let mut db = DATABASE.lock().unwrap();

    if let Err(e) = db.add_user(user) {
        error!("{}", e);
        return (
            http::Status::new(500),
            jobject::Error(e.to_string()).to_string(),
        );
    }

    let snuggled: jobject::User = match db.get_user(&snuggled) {
        Ok(s) => s,
        Err(e) => {
            error!("{}", e);
            return (
                http::Status::new(500),
                jobject::Error(e.to_string()).to_string(),
            );
        }
    }
    .into();

    match serde_json::to_string(&snuggled) {
        Ok(s) => (http::Status::new(200), s),
        Err(e) => {
            error!("{}", e);
            (
                http::Status::new(500),
                jobject::Error(e.to_string()).to_string(),
            )
        }
    }
}

#[get("/fingerprint/<user>/<fingerprint>")]
async fn fingerprint(user: &str, fingerprint: Uuid) -> Result {
    let snuggler: Username = match user.parse() {
        Ok(u) => u,
        Err(e) => {
            error!("{}", e);
            return (
                http::Status::new(500),
                jobject::Error(e.to_string()).to_string(),
            );
        }
    };

    if let Some(uuid) = FEDERATION.lock().unwrap().remove(&snuggler.to_string()) {
        if fingerprint == uuid {
            let mut db = DATABASE.lock().unwrap();

            let snuggler: jobject::User = match db.get_user(&snuggler) {
                Ok(s) => s,
                Err(e) => {
                    error!("{}", e);
                    return (
                        http::Status::new(500),
                        jobject::Error(e.to_string()).to_string(),
                    );
                }
            }
            .into();

            match serde_json::to_string(&snuggler) {
                Ok(s) => (http::Status::new(200), s),
                Err(e) => {
                    error!("{}", e);
                    (
                        http::Status::new(500),
                        jobject::Error(e.to_string()).to_string(),
                    )
                }
            }
        } else {
            (
                http::Status::new(404),
                jobject::Error("invalid fingerprint".to_owned()).to_string(),
            )
        }
    } else {
        (
            http::Status::new(404),
            jobject::Error("fingerprint not found".to_owned()).to_string(),
        )
    }
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
