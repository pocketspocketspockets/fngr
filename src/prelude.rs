use std::path::PathBuf;

pub use anyhow::{Result, anyhow};

use crate::user::Username;
// pub use tracing::{debug, error, info, subscriber, warn};

/// Helper function to return an error if path is relative using `Path::is_relative`. The check is disabled in a debug binary.
#[inline]
#[allow(unused)]
pub fn is_relative(name: &str, p: impl Into<PathBuf>) -> Result<()> {
    let p = p.into();
    #[cfg(not(debug_assertions))]
    if p.is_relative() {
        return Err(anyhow!("{} path cannot be relative!", name));
    }

    #[cfg(debug_assertions)]
    {
        // warn!(
        //     "debug_assertions: skipping relative path check for {}",
        //     name
        // );
    }

    Ok(())
}

// pub fn need_federated_username(username: &Username, server: Option<&str>) -> Result<Username> {
//     if username.is_local() && server.is_none() {
//         Err(anyhow!(
//             "user '{}' is local while server is `None`!",
//             username
//         ))
//     } else if username.is_local() & server.is_some() {
//         Ok(username.to_owned().to_federated(server.unwrap()))
//     } else {
//         Ok(username.to_owned())
//     }
// }
