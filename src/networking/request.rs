use crate::prelude::*;
use anyhow::anyhow;
use std::{collections::HashMap, str::FromStr};
use tokio::io::{AsyncBufRead, AsyncBufReadExt};

pub struct Request {
    pub action: Action,
    pub username: Option<String>,
    pub key: Option<String>,
    pub auth: Option<String>,
    pub finger_user: Option<String>,
    pub status: Option<String>,
    // pub headers: HashMap<String, String>,
}

impl Request {
    pub async fn parse(mut stream: impl AsyncBufRead + Unpin) -> Result<Self> {
        let mut line_buffer = String::new();
        stream.read_line(&mut line_buffer).await?;

        let mut parts = line_buffer.split_whitespace();

        let m = parts.next().ok_or(anyhow!("invalid request type"))?;

        if m != "GET" {
            return Err(anyhow!("invalid request type: '{}'", m));
        }

        let path: String = parts
            .next()
            .ok_or(anyhow!("missing path"))
            .map(Into::into)?;
        let action: Action;
        let mut username = None;
        let mut key = None;
        let mut user = None;
        let mut status = None;

        if path.starts_with("/") {
            let s: Vec<&str> = path.split("?").collect();
            action = s[0][1..].parse()?;

            if s.len() != 1 {
                let s = s[1];
                for a in s.split("&") {
                    let b: Vec<&str> = a.split("=").collect();

                    match b[0] {
                        "username" => username = Some(b[1].to_owned()),
                        "key" => key = Some(b[1].to_owned()),
                        "user" => user = Some(b[1].to_owned()),
                        "status" => status = Some(b[1].to_owned()),
                        _ => {}
                    }
                }
            }
        } else {
            return Err(anyhow!("invalid action:"));
        }

        let mut headers = HashMap::new();

        loop {
            line_buffer.clear();
            stream.read_line(&mut line_buffer).await?;

            if line_buffer.is_empty() || line_buffer == "\n" || line_buffer == "\r\n" {
                break;
            }

            let mut comps = line_buffer.split(":");
            let key = comps.next().ok_or(anyhow!("invalid header"))?;
            let value = comps.next().ok_or(anyhow!("invalid header"))?.trim();
            headers.insert(key.to_string(), value.to_string());
        }

        Ok(Request {
            action,
            username,
            auth: headers.get("Authorization").map(|s| s.to_owned()),
            key,
            finger_user: user,
            status,
            // headers,
        })
    }
}

pub enum Action {
    Login,
    Logoff,
    Finger,
    Check,
    Bump,
    List,
    Register,
    Deregister,
}

impl FromStr for Action {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "finger" => Ok(Self::Finger),
            "login" => Ok(Self::Login),
            "bump" => Ok(Self::Bump),
            "list" => Ok(Self::List),
            "register" => Ok(Self::Register),
            "deregister" => Ok(Self::Deregister),
            "logoff" => Ok(Self::Logoff),
            "check" => Ok(Self::Check),
            _ => Err(anyhow!("unrecognized action '{}'", s)),
        }
    }
}
