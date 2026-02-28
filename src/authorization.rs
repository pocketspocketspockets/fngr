use rocket::{Request, http::Status, request::{self, FromRequest}};
use sha_rs::Sha;

pub struct Authorization(String);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Authorization {
    type Error = anyhow::Error;

    async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let digest = sha_rs::Sha256::new();
        let token = req.headers().get_one("Authorization");

        match token {
            Some(h) => request::Outcome::Success(Authorization(digest.digest(h.as_bytes()))),
            None => request::Outcome::Forward(Status::Processing),
        }
    }
}
