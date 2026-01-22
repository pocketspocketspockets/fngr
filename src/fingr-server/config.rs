use crate::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::{fs::File, io::AsyncReadExt};

pub struct Config {
    pub socket_path: String,
    pub users_list: PathBuf,
    pub registration: bool,
    pub auth_key: Option<String>,
    pub lock: PathBuf,
    // file: File,
}

impl Config {
    pub async fn load(p: Option<PathBuf>) -> Result<Self> {
        let p = if let Some(p) = p {
            is_relative("config", &p)?;
            p
        } else {
            PathBuf::from("/etc/fngr-server/config")
        };

        info!("loading config from {}", p.display());

        let (init, _) = InitialConfig::load(&p).await?;

        let socket_path = format!("{}:{}", init.address, init.port);
        let users_list = PathBuf::from(init.users_list);
        let auth_key = init.auth_key;
        let lock = init.lock;
        // let file = fs;
        let regis = init.registration;

        if auth_key.is_none() && regis {
            warn!("registration is enabled and authentication key is empty: anybody can register")
        }

        Ok(Self {
            socket_path,
            users_list,
            auth_key,
            lock: lock.unwrap_or(PathBuf::from("/var/finger.lock")),
            // file,
            registration: regis,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct InitialConfig {
    address: String,
    port: u16,
    users_list: String,
    registration: bool,
    auth_key: Option<String>,
    lock: Option<PathBuf>,
}

impl InitialConfig {
    async fn load(p: &Path) -> Result<(Self, File)> {
        let mut buffer = vec![];
        let mut file = File::open(p).await?;

        file.read_to_end(&mut buffer).await?;

        Ok((toml::from_slice(&buffer)?, file))
    }
}
