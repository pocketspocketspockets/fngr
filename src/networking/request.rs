use crate::prelude::*;
use anyhow::anyhow;
use std::{collections::HashMap, str::FromStr};
use tokio::io::{AsyncBufRead, AsyncBufReadExt};

#[derive(Debug)]
pub struct Request {
    pub action: Action,
    pub kind: RequestKind,
    pub username: Option<String>,
    pub headers: HashMap<String, String>,
    pub params: HashMap<String, String>,
    pub authorization: Option<String>,
}

impl Request {
    pub async fn parse_buf(mut stream: impl AsyncBufRead + Unpin) -> Result<Self> {
        let mut line_buffer = String::new();
        stream.read_line(&mut line_buffer).await?;

        let mut parts = line_buffer.split_whitespace();

        let kind = parts
            .next()
            .ok_or(anyhow!("invalid request type"))?
            .parse()?;

        let path: String = parts
            .next()
            .ok_or(anyhow!("missing path"))
            .map(Into::into)?;
        let action: Action;

        let mut params = HashMap::new();

        if path.starts_with("/") {
            let s: Vec<&str> = path.split("?").collect();
            action = s[0][1..].parse()?;

            if s.len() != 1 {
                let s = s[1];
                for a in s.split("&") {
                    let b: Vec<&str> = a.split("=").collect();

                    params.insert(b[0].to_owned(), b[1].to_owned());
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

        let authorization = headers.remove("Authorization").or(params.remove("auth"));
        let username = params.remove("username");

        Ok(Request {
            action,
            kind,
            username,
            headers,
            params,
            // content: todo!(),
            authorization,
        })
    }

    pub fn new(
        action: Action,
        kind: RequestKind,
        username: Option<String>,
        headers: HashMap<String, String>,
        params: HashMap<String, String>,
        authorization: Option<String>,
    ) -> Self {
        Self {
            action,
            kind,
            username,
            headers,
            params,
            authorization,
        }
    }
}

#[derive(Debug)]
pub enum RequestKind {
    Get,
    Post,
    Delete,
    Put,
}

impl FromStr for RequestKind {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "GET" => Ok(Self::Get),
            "POST" => Ok(Self::Post),
            "DELETE" => Ok(Self::Delete),
            "PUT" => Ok(Self::Put),
            _ => Err(anyhow!("unknown request type: '{}'", s)),
        }
    }
}

#[derive(Debug)]
pub enum Action {
    Login,
    Logoff,
    Snuggle,
    Check,
    Bump,
    List,
    Register,
    Deregister,
    Info,
    SetBio,
    AddSocial,
    DelSocial,
    SetWeb,
    Fingerprint,
    FedSnuggle,
}

impl FromStr for Action {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "snuggle" => Ok(Self::Snuggle),
            "login" => Ok(Self::Login),
            "bump" => Ok(Self::Bump),
            "list" => Ok(Self::List),
            "register" => Ok(Self::Register),
            "deregister" => Ok(Self::Deregister),
            "logoff" => Ok(Self::Logoff),
            "check" => Ok(Self::Check),
            "setbio" => Ok(Self::SetBio),
            "addsocial" => Ok(Self::AddSocial),
            "delsocial" => Ok(Self::DelSocial),
            "setweb" => Ok(Self::SetWeb),
            "" | "info" => Ok(Self::Info),
            "fed_snuggle" => Ok(Self::FedSnuggle),
            "fingerprint" => Ok(Self::Fingerprint),
            _ => Err(anyhow!("unrecognized action '{}'", s)),
        }
    }
}
