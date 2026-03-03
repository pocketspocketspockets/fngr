use crate::{
    jobject::{self, SnuggleLog, SocialsList},
};
use rusqlite::{
    ToSql,
    types::{FromSql, FromSqlError, Value},
};
use serde::{Deserialize, Serialize};
use std::{
    error::Error,
    fmt::Display,
    str::FromStr,
};
use time::UtcDateTime;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Username {
    username: String,
    server: String,
}

impl Username {
    pub fn server(&self) -> &str {
        &self.server
    }
}

impl Display for Username {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}@{}", self.username, self.server)
    }
}

#[derive(Debug)]
pub struct UsernameParseError {
    username: String,
    info: &'static str,
}

impl Display for UsernameParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: '{}'", self.info, self.username)
    }
}

impl Into<FromSqlError> for UsernameParseError {
    fn into(self) -> FromSqlError {
        FromSqlError::Other(Box::new(self))
    }
}

impl Error for UsernameParseError {}

impl FromStr for Username {
    type Err = UsernameParseError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        if s.contains("@") {
            let mut split = s.split("@");
            let username = split.next().ok_or(UsernameParseError {
                info: "failed to split username from string",
                username: s.to_owned(),
            })?;
            let server = split.next().ok_or(UsernameParseError {
                info: "failed to split server from string",
                username: s.to_owned(),
            })?;
            Ok(Self {username: username.to_owned(), server: server.to_owned() })
        } else {
            Err(UsernameParseError { username: s.to_owned(), info: "unable to parse username" })
        }
    }
}

impl ToSql for Username {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        let s = self.to_string();
        Ok(rusqlite::types::ToSqlOutput::Owned(Value::Text(s)))
    }
}

impl FromSql for Username {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        match value {
            rusqlite::types::ValueRef::Text(items) => {
                let s = String::from_utf8(items.to_vec())
                    .map_err(|e| FromSqlError::Other(Box::new(e)))?;
                let username = s.parse().map_err(Into::into)?;
                Ok(username)
            }
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

// /// # User
// ///
// /// This struct contains user information
pub struct User {
    username: Username,
    hash: Option<String>,
    status: Status,
    log: SnuggleLog,
    website: Option<String>,
    social: SocialsList,
    bio: Option<String>,
}

impl User {
    pub fn new(
        username: Username,
        hash: Option<String>,
        status: Status,
        log: SnuggleLog,
        website: Option<String>,
        social: SocialsList,
        bio: Option<String>,
    ) -> Self {
        Self {
            username,
            hash,
            status,
            log,
            website,
            social,
            bio,
        }
    }
    pub fn username(&self) -> &Username {
        &self.username
    }

    pub fn hash(&self) -> Option<&str> {
        self.hash.as_deref()
    }

    pub fn status(&self) -> &Status {
        &self.status
    }

    pub fn log(&self) -> &SnuggleLog {
        &self.log
    }

    pub fn website(&self) -> Option<&str> {
        self.website.as_deref()
    }

    pub fn social(&self) -> &SocialsList {
        &self.social
    }

    pub fn bio(&self) -> Option<&str> {
        self.bio.as_deref()
    }
}

impl Into<jobject::User> for User {
    fn into(self) -> jobject::User {
        jobject::User {
            username: self.username().to_string(),
            server: self.username().server().to_owned(),
            website: self.website.clone(),
            social: self.social.clone().into(),
            bio: self.bio.clone(),
            status: self.status.clone().into(),
        }
    }
}

// /// User status
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Status {
    /// `true` if online, `false` if offline
    pub online: bool,
    /// Status text
    pub text: Option<String>,
    /// Time since last logon/logoff
    pub bumped: Option<time::UtcDateTime>,
    pub since: time::UtcDateTime,
}

impl Default for Status {
    fn default() -> Self {
        Self {
            online: Default::default(),
            text: Default::default(),
            bumped: Default::default(),
            since: UtcDateTime::now(),
        }
    }
}
