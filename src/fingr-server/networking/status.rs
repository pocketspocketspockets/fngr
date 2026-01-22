use std::fmt::Display;

pub enum ResponseStatus {
    NotFound,
    Ok,
    Unauth,
    Bad,
    ServerError,
}

impl Display for ResponseStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResponseStatus::NotFound => "404 Not Found",
            ResponseStatus::Ok => "200 OK",
            ResponseStatus::Unauth => "401 Unauthorized",
            ResponseStatus::Bad => "400 Bad Request",
            ResponseStatus::ServerError => "500 Server Error",
        }
        .fmt(f)
    }
}
