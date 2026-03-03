use crate::prelude::*;
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub struct Config {
    pub server_name: String,
    pub address: Option<String>,
    pub port: Option<u16>,
    // pub socket_path: String,
    pub database: PathBuf,
    pub registration: bool,
    pub auth_key: Option<String>,
}

impl Config {
    pub fn load(p: Option<impl Clone + Into<PathBuf>>) -> Result<Self> {
        let p: PathBuf = if let Some(p) = p {
            is_relative("config", p.clone())?;
            p.into()
        } else {
            PathBuf::from("/etc/snuggle/config")
        };

        // info!("loading config from {}", p.display());

        let (init, _) = InitialConfig::load(p.clone())?;

        let server_name = init.server_name.clone();
        let address = init.address;
        let port = init.port;
        // let socket_path = format!(
        //     "{}:{}",
        //     init.address.or(Some(server_name.clone())).unwrap(),
        //     init.port
        // );
        let database = PathBuf::from(init.database);
        let auth_key = init.auth_key;
        let regis = init.registration;

        // if auth_key.is_none() && regis {
        //     warn!("registration is enabled and authentication key is empty: anybody can register")
        // }

        Ok(Self {
            server_name,
            address,
            port,
            // socket_path,
            database,
            auth_key,
            registration: regis,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct InitialConfig {
    server_name: String,
    address: Option<String>,
    port: Option<u16>,
    database: String,
    registration: bool,
    auth_key: Option<String>,
    lock: Option<PathBuf>,
}

impl InitialConfig {
    fn load(p: PathBuf) -> Result<(Self, File)> {
        let mut buffer = vec![];
        let mut file = File::open(p)?;

        file.read_to_end(&mut buffer)?;

        Ok((toml::from_slice(&buffer)?, file))
    }
}
