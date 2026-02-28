use std::sync::Mutex;
use lazy_static::lazy_static;
use rocket::{get, post, routes, serde::uuid::Uuid};
use tracing::error;
use crate::{authorization::Authorization, config::Config, jobject::JSONResponse, user::UserList};

mod config;
mod prelude;
mod authorization;
mod user;
mod jobject;

#[cfg(debug_assertions)]
lazy_static! {
    static ref CONFIG: Mutex<Config> = Mutex::new(Config::load(Some("./snuggle.config")).unwrap());
}

#[cfg(not(debug_assertions))]
lazy_static! {
    static ref CONFIG: Mutex<Config> = Mutex::new(Config::load(None).unwrap());
}

lazy_static! {
    static ref DATABASE: Mutex<UserList> = {
        let server_name = CONFIG.lock().unwrap().server_name.clone();
        let path = CONFIG.lock().unwrap().database.clone();
        Mutex::new(UserList::load(&server_name, &path).unwrap())
    };
}

#[get("/info")]
fn info() {
    todo!()
}

#[get("/login?<username>&<auth>&<status>")]
async fn login(username: &str, status: Option<&str>, auth: Option<&str>, hauth: Option<Authorization>) {
}

#[get("/logoff?<username>&<auth>")]
fn logoff(username: &str, auth: Option<&str>, hauth: Authorization) {}

#[get("/snuggle?<username>&<user>&<auth>")]
fn snuggle(username: &str, user: &str, auth: Option<&str>, hauth: Authorization) {}

#[get("/check?<username>&<auth>")]
fn check(username: &str, auth: Option<&str>, hauth: Authorization) {}

#[get("/bump?<username>&<auth>")]
fn bump(username: &str, auth: Option<&str>, hauth: Authorization) {}

#[get("/list")]
async fn list() -> Option<String> {
    let ul = DATABASE.lock().unwrap();
    let mut output: Vec<JSONResponse> = Vec::new();

    for user in ul.values() {
        println!("{:?}", user.username());
        output.push(JSONResponse::TinyUser { username: user.username(), online: user.online() })
    }

    let output = JSONResponse::List(output);
    match serde_json::to_string(&output) {
        Ok(a) => Some(a),
        Err(e) => {
            error!("failed to generate json response: {e}");
            Some(JSONResponse::Error(e.to_string()).to_string())
        },
    }
}

#[get("/register?<username>&<password>")]
fn register(username: &str, password: Option<&str>) {

}

#[get("/deregister?<username>&<auth>")]
fn deregister(username: &str, auth: Option<&str>, hauth: Authorization) {}

#[get("/setbio?<username>&<auth>&<bio>")]
fn set_bio(username: &str, auth: Option<&str>, bio: Option<&str>, hauth: Authorization) {}

#[get("/addsocial?<username>&<auth>&<name>&<string>")]
fn add_social(username: &str, auth: Option<&str>, name: &str, string: &str, hauth: Authorization) {}

#[get("/delsocial?<username>&<auth>&<name>")]
fn del_social(username: &str, auth: Option<&str>, name: &str, hauth: Authorization) {}

#[get("/setweb?<username>&<auth>&<addr>")]
fn website(username: &str, auth: Option<&str>, addr: Option<&str>, hauth: Authorization) {}

#[post("/<fingerprint>/<snuggled>/<from>")]
fn fed_snuggle(fingerprint: Uuid, snuggled: &str, from: &str) {}

#[get("/fingerprint/<user>/<fingerprint>")]
fn fingerprint(user: &str, fingerprint: Uuid) {}

#[rocket::main]
async fn main() {
    let rocket = rocket::build()
        .mount(
            "/",
            routes![
                info,
                login,
                logoff,
                snuggle,
                check,
                bump,
                list,
                register,
                deregister,
                set_bio,
                add_social,
                del_social,
                website,
            ],
        )
        .mount("/fed", routes![fed_snuggle, fingerprint])
        .launch()
        .await
        .unwrap();
}
