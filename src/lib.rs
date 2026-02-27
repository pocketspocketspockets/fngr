//!
//! # Snuggle
//!
//! cozy, welcoming, decentralized, federated min-social network.
//! 
//! ##. Errors
//! 
//! ```
//! { "Error": String }
//! ```

pub mod networking;
pub mod prelude;
pub mod userlist;

use self::networking::{Request, Response};
use self::prelude::*;

pub trait Snuggle {
    type SelfLock;

    /// # Info
    /// 
    /// Returns server's information and API version.
    /// 
    /// ## Endpoint
    /// 
    /// - `/info`   required
    /// - `/`       optional
    /// 
    /// ## Heaader
    /// 
    /// No headers
    /// 
    /// ## Parameters
    /// 
    /// No Parameters
    /// 
    /// ## Response
    /// 
    /// - 200 on success
    /// - 500 on internal error
    /// 
    /// ### 200
    /// 
    /// Responds with Info JSON object.
    /// 
    /// - name: server software name
    /// - version: api version implemented by the server
    /// - license: license of the server software
    /// - contact: server maintainer contact information
    /// - users: array of two ints: online users, total users
    /// 
    /// ```
    /// {
    ///     "Info": {
    ///         "name": "snuggle",
    ///         "version": "0.3.0-alpha",
    ///         "licesnse": "GPL-3",
    ///         "contact": "Avery Murray <pocketsbuzz@pm.me>",
    ///         "users": [1,3]
    ///     }
    /// }
    /// ```
    fn info(state: Self::SelfLock, req: Request) -> impl Future<Output = Result<Response>>;

    /// # Login
    /// 
    /// Allows a user to login. User will be marked online and set offline automatically after 1 hours.
    /// Authorization requied.
    /// 
    /// ## Endpoint
    /// 
    /// - `/login` required
    ///
    /// ## Heaader
    /// 
    /// - `Authorization` with user's passphrase
    /// 
    /// ## Parameters
    /// 
    /// - `username` user's username
    /// - `auth` user's passphrase
    /// 
    ///  ## Response
    /// 
    /// Server can reply with
    /// - 200 on success
    /// - 404 on user isn't found
    /// - 400 on improperly formated request
    /// - 401 on unauthorized
    /// - 500 on server failure
    /// 
    /// ### 200
    /// 
    /// ```
    /// {
    ///     "Ok": String
    /// }
    /// ```
    fn login(state: Self::SelfLock, req: Request) -> impl Future<Output = Result<Response>>;

    /// # Logoff
    /// 
    /// Allows a user to logiff. User will be marked offline.
    /// Authorization requied.
    /// 
    /// ## Endpoint
    /// 
    /// - `/logoff` required
    ///
    /// ## Heaader
    /// 
    /// - `Authorization` with user's passphrase
    /// 
    /// ## Parameters
    /// 
    /// - `username` user's username
    /// - `auth` user's passphrase
    /// 
    ///  ## Response
    /// 
    /// Server can reply with
    /// - 200 on success
    /// - 404 on user isn't found
    /// - 400 on improperly formated request
    /// - 401 on unauthorized
    /// - 500 on server failure
    /// 
    /// ### 200
    /// 
    /// ```
    /// {
    ///     "Ok": String
    /// }
    /// ```
    fn logoff(state: Self::SelfLock, req: Request) -> impl Future<Output = Result<Response>>;

    /// # Snuggle
    /// 
    /// Snuggle a user listed in `user` param. Returning that user's information and adding yourself to their log.
    /// Authorization required.
    /// 
    /// ## Endpoint
    /// 
    /// - `/snuggle` required
    ///
    /// ## Heaader
    /// 
    /// - `Authorization` with user's passphrase
    /// 
    /// ## Parameters
    /// 
    /// - `username` user's username
    /// - `auth` user's passphrase
    /// - `user` user to snuggle's username (`foo`` or `foo@exmaple.com`)
    /// 
    ///  ## Response
    /// 
    /// Server can reply with
    /// - 200 on success
    /// - 404 on user isn't found
    /// - 400 on improperly formated request
    /// - 401 on unauthorized
    /// - 500 on server failure
    /// 
    /// ### 200
    /// 
    /// Returns `User` json object
    /// 
    /// ```
    /// {
    ///   "User": {
    ///     "username": "pockets@s.pockets.buzz",
    ///     "status": {
    ///       "online": true,
    ///       "text": "Heyy~",
    ///       "since": 347
    ///     },
    ///     "website": "https://pockets.buzz/",
    ///     "socials": {
    ///       "github": "https://github.com/pocketspocketspockets"
    ///     },
    ///     "bio": "comfort bun"
    ///   }
    /// }
    /// ```
    fn snuggle(state: Self::SelfLock, req: Request) -> impl Future<Output = Result<Response>>;

    /// # Check
    ///
    /// Check users who snuggled and clears the list.
    /// Authorization required.
    ///
    /// ## Endpoints
    ///
    /// - `/check` required
    ///
    /// ## Headers
    ///
    /// - `Authorization` with user's password.
    ///
    /// ## Parameters
    ///
    /// - `username` with user's username
    /// - `auth` with user's passphrase
    ///
    /// ## Response
    ///
    /// Server can reply with
    /// - 200 on success
    /// - 404 on user isn't found
    /// - 400 on improperly formated request
    /// - 401 on unauthorized
    /// - 500 on server failure
    ///
    /// ### 200
    ///
    /// JSON object json list of  `User` objects.
    /// ```
    /// {
    ///     "List": [
    ///         {
    ///             "User": { ... }
    ///         }
    ///     ]
    /// }
    /// ```
    ///
    fn check(state: Self::SelfLock, req: Request) -> impl Future<Output = Result<Response>>;

    /// # Bump
    ///
    /// Bump's a user to keep them online for longer than the standard hour.
    /// Authorization required.
    /// 
    /// ## Endpoints
    ///
    /// Must be avilable at `/bump`
    ///
    /// ## Headers
    ///
    /// - `Authorization` user's password.
    ///
    /// ## Parameters
    ///
    /// - `username` with user's username
    /// - `auth` with user's passphrase
    ///
    /// ## Response
    ///
    /// Server can reply with
    /// - 200 on success
    /// - 404 on user isn't found
    /// - 400 on improperly formated request
    /// - 401 on unauthorized
    /// - 500 on server failure
    ///
    /// ### 200
    ///
    /// ```
    /// {
    ///     "Ok": String
    /// }
    /// ```
    fn bump(state: Self::SelfLock, req: Request) -> impl Future<Output = Result<Response>>;

    /// # List
    ///
    /// List users on the server
    ///
    /// ## Response
    ///
    /// - 200 on success
    /// - 500 on server failure
    ///
    /// ### 200
    ///
    /// JSON object list with `User` objects
    ///
    /// ```
    /// {
    ///     "List": [
    ///     {
    ///         "User": { ... }
    ///     },
    ///     {
    ///         "User": { ... }
    ///     }]
    /// }
    /// ```
    fn list(state: Self::SelfLock, req: Request) -> impl Future<Output = Result<Response>>;

    /// # Register
    ///
    /// Register a new user account on the server.
    ///
    /// ## Endpoints
    ///
    /// - `/register`
    ///
    /// ## Parameters
    ///
    /// - `key` optional server registration key
    /// - `username` must be provided with new username
    /// - `password` mut be provided with new passphrase
    ///
    /// ## Response
    ///
    /// Server can reply with
    /// - 200 on success
    /// - 404 on user isn't found
    /// - 400 on improperly formated request
    /// - 401 on unauthorized
    /// - 500 on server failure
    ///
    /// ### 200
    ///
    /// ```
    /// {
    ///     "Ok": String
    /// }
    /// ```
    fn register(state: Self::SelfLock, req: Request) -> impl Future<Output = Result<Response>>;

    /// # Deregister
    ///
    /// Remove user account on the server.
    /// Authorization required
    ///
    /// ## Endpoints
    ///
    /// - `/deregister` required
    ///
    /// ## Headers
    ///
    /// - `Authorization` with user's passphrase
    ///
    /// ## Parameters
    ///
    /// - `key` with optional server registration key
    /// - `username` with user's username
    /// - `auth` with user's passphrase
    ///
    /// ## Response
    ///
    /// Server can reply with
    /// - 200 on success
    /// - 404 on user isn't found
    /// - 400 on improperly formated request
    /// - 401 on unauthorized
    /// - 500 on server failure
    ///
    /// ### 200
    ///
    /// ```
    /// {
    ///     "Ok": String
    /// }
    /// ```
    fn deregister(state: Self::SelfLock, req: Request) -> impl Future<Output = Result<Response>>;

    /// # set bio
    /// 
    /// Set or removes a user's biography / description
    /// no `bio` bio to `null`
    /// Authorization
    /// 
    /// ## Endpoint
    /// 
    /// - `/setbio` required.
    /// 
    /// ## Headers
    /// 
    /// - `Authorization` required with user's passphrase
    /// 
    /// ## Parameters
    /// 
    /// - `username` requried with user's username
    /// - `auth` required with user's passphrase
    /// - `bio` optional with a biography, spaces separated with `+`. Ombittion of this parameters sets bio to `null`.
    /// 
    /// ## Response
    ///
    /// Server can reply with
    /// - 200 on success
    /// - 404 on user isn't found
    /// - 400 on improperly formated request
    /// - 401 on unauthorized
    /// - 500 on server failure
    ///
    /// ### 200
    ///
    /// ```
    /// {
    ///     "Ok": String
    /// }
    /// ```
    fn set_bio(state: Self::SelfLock, req: Request) -> impl Future<Output = Result<Response>>;

    /// # add social
    /// 
    /// Adds a social link to a user's profile
    /// Authorization required.
    /// 
    /// ## Endpoint
    /// 
    /// - `/addsocial` required.
    /// 
    /// ## Headers
    /// 
    /// - `Authorization` with user's passphrase
    /// 
    /// ## Parameters
    /// 
    /// - `username` with user's username
    /// - `auth` with user's passphrase
    /// - `name` name of service.
    /// - `string` url of profile on service.
    /// 
    /// ## Response
    ///
    /// Server can reply with
    /// - 200 on success
    /// - 404 on user isn't found
    /// - 400 on improperly formated request
    /// - 401 on unauthorized
    /// - 500 on server failure
    ///
    /// ### 200
    ///
    /// ```
    /// {
    ///     "Ok": String
    /// }
    /// ```
    fn add_social(state: Self::SelfLock, req: Request) -> impl Future<Output = Result<Response>>;

    /// # remove social
    /// 
    /// Removes a social link from a user's profile
    /// Authorization required.
    /// 
    /// ## Endpoint
    /// 
    /// - `/delsocial` required.
    /// 
    /// ## Headers
    /// 
    /// - `Authorization` with user's passphrase
    /// 
    /// ## Parameters
    /// 
    /// - `username` with user's username
    /// - `auth` with user's passphrase
    /// - `name` name of service.
    /// 
    /// ## Response
    ///
    /// Server can reply with
    /// - 200 on success
    /// - 404 on user isn't found
    /// - 400 on improperly formated request
    /// - 401 on unauthorized
    /// - 500 on server failure
    ///
    /// ### 200
    ///
    /// ```
    /// {
    ///     "Ok": String
    /// }
    /// ```
    fn remove_social(state: Self::SelfLock, req: Request)
    -> impl Future<Output = Result<Response>>;

    /// # Set website
    /// 
    /// Adds or removes a website link to a user's profile.
    /// no addr removes the link.
    /// Authorization requied.
    /// 
    /// ## Endpoint
    /// 
    /// `/addsocial` is required.
    /// 
    /// ## Headers
    /// 
    /// - `Authorization` required with user's password
    /// 
    /// ## Parameters
    /// 
    /// - `username` requried with user's username
    /// - `addr` optional with URL. Obmittion sets website to `null`.
    /// 
    /// ## Response
    ///
    /// Server can reply with
    /// - 200 on success
    /// - 404 on user isn't found
    /// - 400 on improperly formated request
    /// - 401 on unauthorized
    /// - 500 on server failure
    ///
    /// ### 200
    ///
    /// ```
    /// {
    ///     "Ok": String
    /// }
    /// ```
    fn set_website(state: Self::SelfLock, req: Request) -> impl Future<Output = Result<Response>>;

    /// # Federated Snuggle (federation)
    /// 
    /// Server-to-server endpoint for snuggling over federation.
    /// 
    /// ## Parameters
    /// 
    /// - `snuggle`: user to be snuggled
    /// - `by_user`: initiating user from original server
    /// - `base`: Uuid fingerprint generated by server for domain verification.
    /// 
    /// ## Response
    ///
    /// Server can reply with
    /// - 200 on success
    /// - 404 on user isn't found
    /// - 400 on improperly formated request
    /// - 401 on unauthorized
    /// - 500 on server failure
    ///
    /// ### 200
    /// 
    /// Responds with `User` object
    fn fed_snuggle(state: Self::SelfLock, req: Request) -> impl Future<Output = Result<Response>>;

    /// # Fingerprint (federation)
    /// 
    /// Server-to-server endpoint for snuggling over federation.
    /// 
    /// ## Parameters
    /// 
    /// - `user`: TODO
    /// - `base`: TODO
    /// 
    /// ## Response
    ///
    /// Server can reply with
    /// - 200 on success
    /// - 404 on user isn't found
    /// - 400 on improperly formated request
    /// - 401 on unauthorized
    /// - 500 on server failure
    ///
    /// ### 200
    /// 
    /// Responds with `User` object
    fn fingerprint(state: Self::SelfLock, req: Request) -> impl Future<Output = Result<Response>>;
}

#[cfg(feature = "blocking")]
mod blocking {
    use super::prelude::*;
    use crate::networking::{Request, Response};

    pub trait Snuggle {
        fn info(&mut self, req: Request) -> Result<Response>;
        fn login(&mut self, req: Request) -> Result<Response>;
        fn logoff(&mut self, req: Request) -> Result<Response>;
        fn finger(&mut self, req: Request) -> Result<Response>;
        fn fed_finger(&mut self, req: Request) -> Result<Response>;
        fn check(&mut self, req: Request) -> Result<Response>;
        fn bump(&mut self, req: Request) -> Result<Response>;
        fn list(&mut self, req: Request) -> Result<Response>;
        fn register(&mut self, req: Request) -> Result<Response>;
        fn deregister(&mut self, req: Request) -> Result<Response>;
        fn set_bio(&mut self, req: Request) -> Result<Response>;
        fn add_social(&mut self, req: Request) -> Result<Response>;
        fn remove_social(&mut self, req: Request) -> Result<Response>;
        fn set_website(&mut self, req: Request) -> Result<Response>;
        fn fingerprint(&mut self, req: Request) -> Result<Response>;
    }
}
