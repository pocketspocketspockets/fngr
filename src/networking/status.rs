use std::fmt::Display;

/// Server status enum
/// 
/// - NotFound 404
/// - Ok 200
/// - Unauth 401
/// - Bad 400
/// - ServerError 500
#[derive(Debug)]
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
