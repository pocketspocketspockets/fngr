use std::{collections::HashMap, fmt::Display, io::Cursor};

use maplit::hashmap;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncWrite, AsyncWriteExt};

use super::status::ResponseStatus;
use crate::{prelude::*, userlist::JSONStatus};

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone)]
pub enum JSONResponse {
    Error(String),
    User {
        username: String,
        status: JSONStatus,
    },
    List(Vec<Self>),
    OK(String),
    Log(Vec<String>),
}

impl Display for JSONResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", serde_json::to_string(self).unwrap())
    }
}

pub struct Response {
    status: ResponseStatus,
    headers: HashMap<String, String>,
    data: Cursor<Vec<u8>>,
}

impl Response {
    pub fn from(status: ResponseStatus, data: impl ToString) -> Self {
        let bytes: Vec<u8> = data.to_string().bytes().collect();

        let headers = hashmap! {
            "Content-Type".to_owned() => "text/html".to_string(),
            "Content-Length".to_owned() => bytes.len().to_string(),
        };

        Self {
            status,
            headers,
            data: Cursor::new(bytes),
        }
    }

    fn status_and_headers(&self) -> String {
        let headers = self
            .headers
            .iter()
            .map(|(k, v)| format!("{}: {}", k, v))
            .collect::<Vec<_>>()
            .join("\r\n");

        format!("HTTP/1.1 {}\r\n{headers}\r\n\r\n", self.status)
    }

    pub async fn write<O: AsyncWrite + Unpin>(mut self, stream: &mut O) -> Result<()> {
        stream
            .write_all(self.status_and_headers().as_bytes())
            .await?;
        tokio::io::copy(&mut self.data, stream).await?;
        Ok(())
    }
}
