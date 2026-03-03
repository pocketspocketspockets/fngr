use rocket::{
    Request, http,
    request::{self, FromRequest},
};
use sha_rs::Sha;

pub struct Authorization(String);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Authorization {
    type Error = anyhow::Error;

    async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        // let digest = sha_rs::Sha256::new();
        let token = req.headers().get_one("Authorization");

        match token {
            Some(h) => request::Outcome::Success(Authorization::from_str(h)),
            None => request::Outcome::Forward(http::Status::Processing),
        }
    }
}

impl ToString for Authorization {
    fn to_string(&self) -> String {
        self.0.clone()
    }
}

impl Authorization {
    pub fn from_str(s: &str) -> Self {
        let r = sha_rs::Sha256::new();
        Authorization(r.digest(s.as_bytes()))
    }
}
