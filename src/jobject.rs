use std::{collections::HashMap, fmt::Display};

use serde::{Deserialize, Serialize};

use crate::user::JSONStatus;

/// Server response JSON objects
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone)]
pub enum JSONResponse {
    /// `{ "error": String }``
    Error(String),
    /// `{ "user": { username: String, "status": { ... }, "website": String?, "socials": [ "foo": "https://bar.com" ], bio: String? } }``
    /// 
    /// for status see `snuggle::userlist::JSONStatus``
    User {
        username: String,
        status: JSONStatus,
        website: Option<String>,
        socials: HashMap<String, String>,
        bio: Option<String>,
    },
    /// List of other JSONReponses
    TinyUser {
        username: String,
        online: bool,
    },
    List(Vec<Self>),
    
    /// Ok response with success information
    /// 
    /// `{ "ok": String }`
    Ok(String),

    /// List for `log`
    Log(Vec<String>),

    /// Server information
    Info {
        name: String,
        version: String,
        licesnse: String,
        contact: String,
        users: (usize, usize),
    },
}

impl Display for JSONResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", serde_json::to_string(self).unwrap())
    }
}
