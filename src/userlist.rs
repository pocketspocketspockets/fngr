use std::{
    collections::HashMap,
    fmt::Display,
    ops::{Deref, DerefMut},
    path::Path,
    time::Duration,
};

use crate::{networking::JSONResponse, prelude::*};
use serde::{Deserialize, Serialize};
use sha_rs::{Sha, Sha256};
use tokio::{
    fs::{File, OpenOptions},
    io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt},
    time::Instant,
};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct JSONStatus {
    online: bool,
    text: Option<String>,
    since: u64,
}

impl Default for JSONStatus {
    fn default() -> Self {
        Status::default().into()
    }
}

impl From<Status> for JSONStatus {
    fn from(value: Status) -> Self {
        Self {
            online: value.online,
            text: value.text,
            since: value.since.elapsed().as_secs(),
        }
    }
}

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
    hash: String,
    status: Status,
    bumped: Option<Instant>,
    log: Vec<JSONResponse>,
    website: Option<String>,
    social: HashMap<String, String>,
    bio: Option<String>,
}

impl Into<JSONResponse> for User {
    fn into(self) -> JSONResponse {
        JSONResponse::User {
            username: self.username.to_owned(),

            status: self.status.into(),
        }
    }
}

impl Into<JSONResponse> for &User {
    fn into(self) -> JSONResponse {
        JSONResponse::User {
            username: self.username.to_owned(),

            status: self.status.clone().into(),
        }
    }
}

impl Into<JSONResponse> for &mut User {
    fn into(self) -> JSONResponse {
        JSONResponse::User {
            username: self.username.to_owned(),

            status: self.status.clone().into(),
        }
    }
}

impl User {
    pub fn username(&self) -> &str {
        &self.username
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

    pub fn compare_key(&self, key: String) -> bool {
        let hasher = Sha256::new();
        let hash = hasher.digest(key.as_bytes());
        hash == self.hash
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
    hash: String,
    website: Option<String>,
    socials: HashMap<String, String>,
    bio: Option<String>,
}

#[cfg(debug_assertions)]
impl Default for InitialUser {
    fn default() -> Self {
        Self {
            username: "null".to_owned(),
            hash: "nope".to_owned(),
            website: None,
            socials: HashMap::new(),
            bio: None,
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
        let s = JSONStatus {
            online: self.online,
            text: self.text.to_owned(),
            since: self.since.elapsed().as_secs(),
        };

        let output = serde_json::to_string(&s).unwrap();

        write!(f, "{}", output)
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
        let mut fin = Self::default();
        let mut users: Vec<InitialUser> = Vec::new();

        if p.exists() {
            let mut file = File::open(p).await?;
            let mut buffer = vec![];
            file.read_to_end(&mut buffer).await?;

            if !buffer.is_empty() {
                users = serde_json::from_slice(&buffer)?;
            }
        } else {
            tokio::fs::File::create_new(p).await?;
        }

        for user in users {
            fin.0.insert(
                user.username.to_owned(),
                User {
                    username: user.username.clone(),
                    hash: match user.hash.parse() {
                        Ok(uuid) => uuid,
                        Err(e) => {
                            return Err(anyhow!(
                                "failed to parse uuid '{}' for user '{}': {e}",
                                user.hash,
                                user.username
                            ));
                        }
                    },
                    status: Status::default(),
                    bumped: None,
                    log: Vec::new(),
                    website: None,
                    social: HashMap::new(),
                    bio: None,
                },
            );
        }

        info!("loaded {} users", fin.len());

        Ok(fin)
    }

    pub async fn register(
        &mut self,
        username: String,
        ulpath: &Path,
        password: Option<&String>,
    ) -> Result<()> {
        if self.contains_key(&username) {
            return Err(anyhow!("username already taken"));
        }

        let password = if let Some(p) = password {
            p
        } else {
            return Err(anyhow!("a password is required"));
        };

        // let uuid = Uuid::from_bytes(rand::random());
        let hasher = Sha256::new();
        let hash = hasher.digest(password.as_bytes());

        let init_user = InitialUser {
            username,
            hash: hash.to_owned(),
            website: None,
            socials: HashMap::new(),
            bio: None,
        };

        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(ulpath)
            .await?;
        let mut buffer = vec![];
        file.read_to_end(&mut buffer).await?;
        file.rewind().await?;

        let users = 
        if !buffer.trim_ascii().is_empty() {
            let mut users: Vec<InitialUser> = serde_json::from_slice(&buffer)?;
            users.push(init_user.clone());
            users
        } else {
            let mut users = Vec::new();
            users.push(init_user.clone());
            users
        };

        let new = serde_json::to_string_pretty(&users)?;

        file.write_all(new.as_bytes()).await?;
        file.flush().await?;

        self.insert(
            init_user.username.to_owned(),
            User {
                username: init_user.username,
                hash,
                status: Status::default(),
                bumped: None,
                log: Vec::new(),
                website: init_user.website,
                social: init_user.socials,
                bio: init_user.bio,
            },
        );

        Ok(())
    }

    pub async fn remove(&mut self, username: String, ulpath: &Path) -> Result<()> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(ulpath)
            .await?;
        let mut buffer = vec![];
        file.read_to_end(&mut buffer).await?;
        file.rewind().await?;
        let mut users: Vec<InitialUser> = serde_json::from_slice(&buffer)?;

        let users_clone = users.clone();
        for (i, user) in users_clone.iter().enumerate() {
            if user.username == username {
                users.remove(i);
            }
        }

        let new = serde_json::to_string_pretty(&users)?;
        file.set_len(0).await?;
        file.write_all(new.as_bytes()).await?;
        file.flush().await?;

        self.0
            .remove(&username)
            .ok_or(anyhow!("failed to remove user"))?;

        Ok(())
    }
}

impl Default for UserList {
    fn default() -> Self {
        Self(HashMap::default())
    }
}
