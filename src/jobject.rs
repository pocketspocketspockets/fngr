use std::{collections::HashMap, fmt::Display};

use rusqlite::{
    ToSql,
    types::{FromSql, FromSqlError, ToSqlOutput},
};
use serde::{Deserialize, Serialize};
use time::UtcDateTime;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Status {
    pub(crate) online: bool,
    pub(crate) text: Option<String>,
    pub(crate) bump: Option<i64>,
    pub(crate) since: i64,
}

impl From<crate::user::Status> for Status {
    fn from(value: crate::user::Status) -> Self {
        Self {
            online: value.online,
            text: value.text,
            bump: value.bumped.map(|i| i.unix_timestamp()),
            since: value.since.unix_timestamp(),
        }
    }
}

impl Into<crate::user::Status> for Status {
    fn into(self) -> crate::user::Status {
        crate::user::Status {
            online: self.online,
            text: self.text,
            // TODO: maybe don't reset time to NOW.
            since: time::UtcDateTime::from_unix_timestamp(self.since)
                .unwrap_or_else(|_| UtcDateTime::now()),
            bumped: self.bump.map(|i| {
                time::UtcDateTime::from_unix_timestamp(i).unwrap_or_else(|_| UtcDateTime::now())
            }),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct User {
    pub username: String,
    pub server: String,
    pub website: Option<String>,
    pub social: SocialsList,
    pub bio: Option<String>,
    pub status: Status,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SnuggleLog(Vec<String>);

impl Default for SnuggleLog {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl ToSql for SnuggleLog {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        let p = serde_json::to_string(self)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
        Ok(ToSqlOutput::Owned(rusqlite::types::Value::Text(p)))
    }
}

impl FromSql for SnuggleLog {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        match value {
            rusqlite::types::ValueRef::Text(items) => {
                let v =
                    serde_json::from_slice(items).map_err(|e| FromSqlError::Other(Box::new(e)))?;
                Ok(v)
            }
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SocialsList(pub HashMap<String, String>);

impl Default for SocialsList {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl ToSql for SocialsList {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        let p = serde_json::to_string(self)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
        Ok(ToSqlOutput::Owned(rusqlite::types::Value::Text(p)))
    }
}

impl Into<HashMap<String, String>> for SocialsList {
    fn into(self) -> HashMap<String, String> {
        self.0
    }
}

impl FromSql for SocialsList {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        match value {
            rusqlite::types::ValueRef::Text(items) => {
                let v =
                    serde_json::from_slice(items).map_err(|e| FromSqlError::Other(Box::new(e)))?;
                Ok(v)
            }
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Info {
    pub name: &'static str,
    pub version: &'static str,
    pub api_version: &'static str,
    pub license: &'static str,
    pub contact: &'static str,
    pub users: [usize; 2],
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Error(pub String);

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{ \"error\": \"{}\" }}", self.0)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserListUser {
    pub username: String,
    pub status: Status,
}
