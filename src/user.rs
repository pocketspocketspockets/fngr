use std::{collections::HashMap, fmt::Display, fs::{File, OpenOptions}, io::{Read, Seek, Write}, ops::{Deref, DerefMut}, path::Path};
use crate::{jobject::JSONResponse, prelude::*};
use rocket::{tokio::time::{Instant, Duration}};
use serde::{Deserialize, Serialize};
use sha_rs::{Sha, Sha256};

/// JSON (De)seriaalizable object with user's status information
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

/// # UserList
/// 
/// struct containing a map of user accounts on the server.
pub struct UserList(HashMap<String, User>);

impl UserList {
    /// Chceks all users and marks them offline after an hour without being bumped.
    pub fn check_statuses(&mut self) {
        for (_, user) in &mut self.0 {
            user.check_status();
        }
    }

    /// Counts total users and online users: retuens (Online, Total)
    pub fn count(&self) -> (usize, usize) {
        let mut online = 0;
        let total = self.0.len();

        for user in &self.0 {
            if user.1.status.online {
                online += 1;
            }
        }

        (online, total)
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

/// # User
/// 
/// This struct contains user information
pub struct User {
    username: String,
    server: String,
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
            username: self.username(),
            status: self.status.into(),
            website: self.website,
            socials: self.social,
            bio: self.bio,
        }
    }
}

impl Into<JSONResponse> for &User {
    fn into(self) -> JSONResponse {
        JSONResponse::User {
            username: self.username(),
            status: self.status.clone().into(),
            website: self.website.clone(),
            socials: self.social.clone(),
            bio: self.bio.clone(),
        }
    }
}

impl Into<JSONResponse> for &mut User {
    fn into(self) -> JSONResponse {
        JSONResponse::User {
            username: self.username(),
            status: self.status.clone().into(),
            website: self.website.clone(),
            socials: self.social.clone(),
            bio: self.bio.clone(),
        }
    }
}

impl Into<InitialUser> for User {
    fn into(self) -> InitialUser {
        InitialUser {
            username: self.username,
            hash: self.hash,
            website: self.website,
            socials: self.social,
            bio: self.bio,
        }
    }
}

impl Into<InitialUser> for &User {
    fn into(self) -> InitialUser {
        InitialUser {
            username: self.username.to_owned(),
            hash: self.hash.to_owned(),
            website: self.website.to_owned(),
            socials: self.social.to_owned(),
            bio: self.bio.to_owned(),
        }
    }
}

impl Into<InitialUser> for &mut User {
    fn into(self) -> InitialUser {
        InitialUser {
            username: self.username.to_owned(),
            hash: self.hash.to_owned(),
            website: self.website.to_owned(),
            socials: self.social.to_owned(),
            bio: self.bio.to_owned(),
        }
    }
}

impl User {
    /// Returns the federated username of the user. `username@server_name.tld``
    pub fn username(&self) -> String {
        let mut username = self.username.clone();
        username = username + "@";
        username = username + &self.server;
        username
    }

    /// Returns reference to the user's status
    pub fn status(&self) -> &Status {
        &self.status
    }

    /// Set the status of the user
    pub fn set_status(&mut self, s: Status) {
        self.status = s;
    }

    /// returns the online status of the user.  `true` for online `false` for offline.
    pub fn online(&self) -> bool {
        self.status.online
    }

    /// returns `true` if the user has been bumped
    fn bumped(&self) -> bool {
        self.bumped.is_some()
    }

    /// bumps the user to keep them marked as online
    pub fn bump(&mut self) -> bool {
        if self.online() {
            self.bumped = Some(Instant::now());
            self.bumped()
        } else {
            false
        }
    }

    /// returns duration since last logon/logoff
    fn time_since(&self) -> Duration {
        self.status.since.elapsed()
    }

    /// verify user's authorization
    pub fn compare_key(&self, key: String) -> bool {
        let hasher = Sha256::new();
        let hash = hasher.digest(key.as_bytes());
        hash == self.hash
    }

    /// marks the user offline if last login/bump was over an hour ago
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

    /// add snuggling user to snuggled user's snuggle log
    pub fn add_log(&mut self, user: JSONResponse) {
        self.log.push(user);
        self.log.dedup();
    }

    /// takes and return log list
    pub fn log(&mut self) -> Vec<JSONResponse> {
        let log = self.log.clone();
        self.log = Vec::new();
        log
    }

    /// sets a website in user profile
    pub fn set_website(&mut self, addr: Option<String>) {
        self.website = addr
    }

    /// adds a social link to user profile
    pub fn add_social(&mut self, name: String, s: String) {
        self.social.insert(name, s);
    }

    /// removes a social link from user profile
    pub fn remove_social(&mut self, name: String) {
        if self.social.contains_key(&name) {
            self.social.remove(&name);
        }
    }

    /// sets a bio to user profile
    pub fn set_bio(&mut self, bio: Option<String>) {
        self.bio = bio.map(|s| s.replace("+", " "));
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone)]
pub(crate) struct InitialUser {
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

/// User status
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Status {
    /// `true` if online, `false` if offline
    pub online: bool,
    /// Status text
    pub text: Option<String>,
    /// Time since last logon/logoff
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
    /// loads the userlist from `path`
    pub fn load(server_name: &str, p: &Path) -> Result<Self> {
        // info!("loading users from {}", p.display());
        is_relative("userlist", p)?;
        let mut fin = Self::default();
        let mut users: Vec<InitialUser> = Vec::new();

        if p.exists() {
            let mut file = File::open(p)?;
            let mut buffer = vec![];
            file.read_to_end(&mut buffer)?;

            if !buffer.is_empty() {
                users = serde_json::from_slice(&buffer)?;
            }
        } else {
            std::fs::File::create_new(p)?;
        }

        for user in users {
            fin.0.insert(
                user.username.to_owned(),
                User {
                    username: user.username.clone(),
                    server: server_name.to_string(),
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

        // info!("loaded {} users", fin.len());

        Ok(fin)
    }

    /// turns self into a vector of `InitialUser` to save user list to disk
    async fn revert(&self) -> Vec<InitialUser> {
        let mut v = Vec::new();

        for (_, user) in &self.0 {
            v.push(user.into());
        }

        v
    }

    /// write userlist to disk
    pub async fn save(&mut self, ulpath: &Path) -> Result<()> {
        let v = self.revert().await;

        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(ulpath)?;

        let new = serde_json::to_string_pretty(&v)?;

        file.write_all(new.as_bytes())?;
        file.flush()?;

        Ok(())
    }

    /// add user to userlist and save to disk
    pub async fn register(
        &mut self,
        username: String,
        ulpath: &Path,
        password: Option<&String>,
        server: String,
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
            .open(ulpath)?;
        let mut buffer = vec![];
        file.read_to_end(&mut buffer)?;
        file.rewind()?;

        let users = if !buffer.trim_ascii().is_empty() {
            let mut users: Vec<InitialUser> = serde_json::from_slice(&buffer)?;
            users.push(init_user.clone());
            users
        } else {
            let mut users = Vec::new();
            users.push(init_user.clone());
            users
        };

        let new = serde_json::to_string_pretty(&users)?;

        file.write_all(new.as_bytes())?;
        file.flush()?;

        self.insert(
            init_user.username.to_owned(),
            User {
                username: init_user.username,
                server: server,
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

    /// remove user from userlist and save to disk
    pub async fn remove(&mut self, username: String, ulpath: &Path) -> Result<()> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(ulpath)?;
        let mut buffer = vec![];
        file.read_to_end(&mut buffer)?;
        file.rewind()?;
        let mut users: Vec<InitialUser> = serde_json::from_slice(&buffer)?;

        let users_clone = users.clone();
        for (i, user) in users_clone.iter().enumerate() {
            if user.username == username {
                users.remove(i);
            }
        }

        let new = serde_json::to_string_pretty(&users)?;
        file.set_len(0)?;
        file.write_all(new.as_bytes())?;
        file.flush()?;

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
