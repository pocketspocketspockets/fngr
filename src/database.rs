use crate::{
    jobject::{self, SnuggleLog},
    prelude::*,
    user::{Status, User, Username},
};
use rusqlite::Connection;
use std::path::Path;
use time::UtcDateTime;

pub struct Database(Connection);

impl Database {
    pub fn load(p: &Path) -> anyhow::Result<Self> {
        is_relative("database", p)?;

        if p.exists() {
            Ok(Self(Connection::open(p)?))
        } else {
            let db = Connection::open(p)?;
            db.execute(
                "CREATE TABLE user (
                    username TEXT PRIMARY KEY,
                    hash TEXT,
                    online INTEGER NOT NULL,
                    message TEXT,
                    since INTEGER NOT NULL,
                    bumped INTEGER,
                    log TEXT,
                    website TEXT,
                    social TEXT,
                    bio TEXT
                        )",
                (),
            )?;
            Ok(Self(db))
        }
    }
    pub fn add_user(&mut self, user: User) -> Result<()> {
        self.0.execute(
            "INSERT INTO user (username, hash, online, message, since, bumped, log, website, social, bio) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            (
                user.username(), user.hash(), user.status().online, user.status().text.clone(), user.status().since.unix_timestamp(), user.status().bumped.map(|t| t.unix_timestamp()), user.log(), user.website(), user.social(), user.bio())
            )?;

        Ok(())
    }

    pub fn delete_user(&mut self, user: &Username) -> Result<()> {
        self.0
            .execute("DELETE FROM user WHERE username = ?1", (user,))?;
        Ok(())
    }

    pub fn get_user(&mut self, username: &Username) -> Result<User> {
        let mut map = self.0.prepare("SELECT username, hash, online, message, since, bumped, log, website, social, bio FROM user WHERE hash IS NOT NULL")?;
        let mut person = map.query_map([], |row| {
            let name: Username = row.get("username")?;
            if *username == name {
                let status = jobject::Status {
                    online: row.get("online")?,
                    text: row.get("message")?,
                    bump: row.get("bumped")?,
                    since: row.get("since")?,
                };
                return Ok(User::new(
                    row.get("username")?,
                    row.get("hash")?,
                    status.into(),
                    row.get("log")?,
                    row.get("website")?,
                    row.get("social")?,
                    row.get("bio")?,
                ));
            }
            Err(rusqlite::Error::InvalidParameterName(username.to_string()))
        })?;

        let person = person.next();
        Ok(person.unwrap()?)
    }

    pub fn get_internal_users(&mut self) -> Result<Vec<User>> {
        let mut map = self.0.prepare("SELECT username, hash, online, message, since, bumped, log, website, social, bio FROM user WHERE hash IS NOT NULL")?;
        let users = map.query_map([], |row| {
            let status = jobject::Status {
                online: row.get("online")?,
                text: row.get("message")?,
                bump: row.get("bumped")?,
                since: row.get("since")?,
            };
            Ok(User::new(
                row.get("username")?,
                row.get("hash")?,
                status.into(),
                row.get("log")?,
                row.get("website")?,
                row.get("social")?,
                row.get("bio")?,
            ))
        })?;

        let mut new_map: Vec<User> = Vec::new();

        for user in users {
            if let Ok(user) = user {
                new_map.push(user)
            }
        }

        Ok(new_map)
    }

    pub fn get_external_users(&mut self) -> Result<Vec<User>> {
        let mut map = self.0.prepare("SELECT username, hash, online, message, since, bumped, log, website, social, bio FROM user WHERE hash IS NULL")?;
        let users = map.query_map([], |row| {
            let status = jobject::Status {
                online: row.get("online")?,
                text: row.get("message")?,
                bump: row.get("bumped")?,
                since: row.get("since")?,
            };
            Ok(User::new(
                row.get("username")?,
                row.get("hash")?,
                status.into(),
                row.get("log")?,
                row.get("website")?,
                row.get("social")?,
                row.get("bio")?,
            ))
        })?;

        let mut new_map: Vec<User> = Vec::new();

        for user in users {
            if let Ok(user) = user {
                new_map.push(user)
            }
        }

        Ok(new_map)
    }

    pub fn count(&mut self) -> Result<[usize; 2]> {
        let users = self.get_internal_users()?;
        let total = users.len();
        let mut online = 0;

        for user in users {
            if user.status().online {
                online += 1;
            }
        }

        Ok([online, total])
    }

    pub fn get_status(&mut self, username: &Username) -> Result<Status> {
        let user = self.get_user(&username)?;
        Ok(user.status().clone())
    }

    pub fn set_user_status_message(&mut self, username: &Username, status: &str) -> Result<()> {
        self.0.execute(
            "UPDATE user SET message = ?1 WHERE username = ?2",
            (status, username),
        )?;

        Ok(())
    }

    pub fn set_user_status_state(
        &mut self,
        username: &Username,
        status: bool,
        since: UtcDateTime,
    ) -> Result<()> {
        self.0.execute(
            "UPDATE user SET status = ?1, since = ?2, bumped = NULL WHERE username = ?3",
            (status, since.unix_timestamp(), username),
        )?;

        Ok(())
    }

    pub fn set_user_status_bump(
        &mut self,
        username: &Username,
        bump: Option<UtcDateTime>,
    ) -> Result<()> {
        self.0.execute(
            "UPDATE user SET bumped = ?1, bumped = NULL WHERE username = ?2",
            (bump.map(|i| i.unix_timestamp()), username),
        )?;

        Ok(())
    }

    pub fn set_user_website(&mut self, username: &Username, website: Option<&str>) -> Result<()> {
        self.0.execute(
            "UPDATE user SET website = ?1 WHERE username = ?2",
            (website, username),
        )?;
        Ok(())
    }

    pub fn set_user_bio(&mut self, username: &Username, bio: Option<&str>) -> Result<()> {
        self.0.execute(
            "UPDATE user SET bio = ?1 WHERE username = ?2",
            (bio, username),
        )?;
        Ok(())
    }

    fn get_user_social(&mut self, username: &Username) -> Result<jobject::SocialsList> {
        let user = self.get_user(username)?;
        Ok(user.social().to_owned())
    }

    fn set_user_social(&mut self, username: &Username, social: jobject::SocialsList) -> Result<()> {
        self.0.execute(
            "UPDATE user SET social = ?1 WHERE username = ?2",
            (social, username),
        )?;
        Ok(())
    }

    pub fn add_to_log(&mut self, username: &Username, by: &Username) -> Result<()> {
        let user = self.get_user(username)?;
        let mut log: SnuggleLog = user.log().clone();
        log.0.push(by.clone());

        self.0.execute(
            "UPDATE user SET log = ?1 WHERE username = ?2",
            (log, username),
        )?;
        Ok(())
    }

    pub fn reset_log(&mut self, username: &Username) -> Result<SnuggleLog> {
        let user = self.get_user(username)?;
        let log: SnuggleLog = user.log().clone();
        let new = SnuggleLog::default();

        self.0.execute(
            "UPDATE user SET log = ?1 WHERE username = ?2",
            (new, username),
        )?;
        Ok(log)
    }

    pub fn add_user_social(&mut self, username: &Username, social: (&str, &str)) -> Result<()> {
        let mut socials = self.get_user_social(&username)?;
        socials.0.insert(social.0.to_owned(), social.1.to_owned());
        self.set_user_social(&username, socials)
    }

    pub fn remove_user_social(&mut self, username: &Username, social: &str) -> Result<()> {
        let mut socials = self.get_user_social(&username)?;
        socials.0.remove(social);
        self.set_user_social(&username, socials)
    }

    pub fn authorize(&mut self, username: &Username, ihash: String) -> Result<()> {
        let user = self.get_user(username)?;
        if let Some(hash) = user.hash() {
            if hash == ihash {
                Ok(())
            } else {
                Err(anyhow!("incorrect passphrase"))
            }
        } else {
            Err(anyhow!(
                "you must login to your own server: '{}'",
                username.server()
            ))
        }
    }
}
