use std::{
    collections::HashMap,
    fmt::Display,
    ops::{Deref, DerefMut, Index},
    path::Path,
    time::Duration,
};

use crate::{networking::JSONResponse, prelude::*};
use serde::{Deserialize, Serialize};
use tinyrand::{Rand, Seeded, StdRand};
use tinyrand_std::ClockSeed;
use tokio::{
    fs::{File, OpenOptions},
    io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt},
    time::Instant,
};
use uuid::Uuid;

pub struct UserList(HashMap<String, User>);

impl UserList {
    pub fn check_statuses(&mut self) {
        for (_, user) in &mut self.0 {
            user.check_status();
        }
    }
}

impl Deref for UserList {
    type Target = HashMap<String, User>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for UserList {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Display for User {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let json: JSONResponse = self.into();
        write!(f, "{}", json)
    }
}

pub struct User {
    username: String,
    uuid: Uuid,
    status: Status,
    bumped: Option<Instant>,
    log: Vec<JSONResponse>,
}

impl Into<JSONResponse> for User {
    fn into(self) -> JSONResponse {
        JSONResponse::User {
            username: self.username.to_owned(),
            online: self.status.online,
            status: self.status.text.to_owned(),
        }
    }
}

impl Into<JSONResponse> for &User {
    fn into(self) -> JSONResponse {
        JSONResponse::User {
            username: self.username.to_owned(),
            online: self.status.online,
            status: self.status.text.to_owned(),
        }
    }
}

impl Into<JSONResponse> for &mut User {
    fn into(self) -> JSONResponse {
        JSONResponse::User {
            username: self.username.to_owned(),
            online: self.status.online,
            status: self.status.text.to_owned(),
        }
    }
}

impl User {
    pub fn username(&self) -> &str {
        &self.username
    }

    fn uuid(&self) -> Uuid {
        self.uuid
    }

    pub fn status(&self) -> &Status {
        &self.status
    }

    pub fn set_status(&mut self, s: Status) {
        self.status = s;
    }

    pub fn online(&self) -> bool {
        self.status.online
    }

    fn bumped(&self) -> bool {
        self.bumped.is_some()
    }

    pub fn bump(&mut self) -> bool {
        if self.online() {
            self.bumped = Some(Instant::now());
            self.bumped()
        } else {
            false
        }
    }

    fn time_since(&self) -> Duration {
        self.status.since.elapsed()
    }

    pub fn compare_key(&self, key: Uuid) -> bool {
        key == self.uuid
    }

    fn check_status(&mut self) {
        match (self.status.online, self.time_since().as_secs(), self.bumped) {
            (true, 3600.., None) => {
                self.status.online = false;
                self.status.since = Instant::now()
            }
            (true, 3600.., Some(s)) => {
                if s.elapsed().as_secs() >= 3600 {
                    self.bumped = None;
                    self.status.since = Instant::now();
                    self.status.online = false
                }
            }
            _ => {}
        }
    }

    pub fn add_log(&mut self, user: JSONResponse) {
        self.log.push(user);
        self.log.dedup();
    }

    pub fn log(&mut self) -> Vec<JSONResponse> {
        let log = self.log.clone();
        self.log = Vec::new();
        log
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct InitialUser {
    username: String,
    uid: String,
}

#[cfg(debug_assertions)]
impl Default for InitialUser {
    fn default() -> Self {
        Self {
            username: "pockets".to_owned(),
            uid: "whaa".to_owned(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Status {
    pub online: bool,
    pub text: Option<String>,
    pub since: Instant,
}

impl Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(status) = &self.text {
            write!(
                f,
                "'online': {}, 'since': '{}', status: '{}'",
                self.online,
                self.since.elapsed().as_secs(),
                status
            )
        } else {
            write!(
                f,
                "'online': {}, 'since': '{}'",
                self.online,
                self.since.elapsed().as_secs()
            )
        }
    }
}

impl Status {
    fn default() -> Status {
        Self {
            online: false,
            text: None,
            since: Instant::now(),
        }
    }
}

impl UserList {
    pub async fn load(p: &Path) -> Result<Self> {
        info!("loading users from {}", p.display());
        is_relative("userlist", p)?;

        let mut file = File::open(p).await?;

        let mut buffer = vec![];
        file.read_to_end(&mut buffer).await?;

        let users: Vec<InitialUser> = serde_json::from_slice(&buffer)?;

        let mut fin = Self::default();

        for user in users {
            fin.0.insert(
                user.username.to_owned(),
                User {
                    username: user.username.clone(),
                    uuid: match user.uid.parse() {
                        Ok(uuid) => uuid,
                        Err(e) => {
                            return Err(anyhow!(
                                "failed to parse uuid '{}' for user '{}': {e}",
                                user.uid,
                                user.username
                            ));
                        }
                    },
                    status: Status::default(),
                    bumped: None,
                    log: Vec::new(),
                },
            );
        }

        info!("loaded {} users", fin.len());

        Ok(fin)
    }

    pub async fn register(&mut self, username: String, ulpath: &Path) -> Result<Uuid> {
        if self.contains_key(&username) {
            return Err(anyhow!("username already taken"));
        }

        let uuid = Uuid::from_bytes(rand::random());
        let init_user = InitialUser {
            username,
            uid: uuid.to_string(),
        };

        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(ulpath)
            .await?;
        let mut buffer = vec![];
        file.read_to_end(&mut buffer).await?;
        file.rewind().await?;

        let mut users: Vec<InitialUser> = serde_json::from_slice(&buffer)?;
        users.push(init_user.clone());

        let new = serde_json::to_string_pretty(&users)?;

        file.write_all(new.as_bytes()).await?;
        file.flush().await?;

        self.insert(
            init_user.username.to_owned(),
            User {
                username: init_user.username,
                uuid,
                status: Status::default(),
                bumped: None,
                log: Vec::new(),
            },
        );

        Ok(uuid)
    }
}

impl Default for UserList {
    fn default() -> Self {
        Self(HashMap::default())
    }
}
