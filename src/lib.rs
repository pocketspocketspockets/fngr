//!
//! # Snuggle
//!
//! cozy, welcoming, decentralized, federated min-social network.
//!

pub mod networking;
pub mod prelude;
pub mod userlist;

use self::networking::{Request, Response};
use self::prelude::*;

pub trait Snuggle {
    type SelfLock;

    /// # Info
    ///
    /// endpoint for receiving server information.
    ///
    /// ## Endpoints
    ///
    /// - Must be available at `/info`
    /// - Optionally available at `/`.
    ///
    /// ## Responses
    ///
    /// Server can reply with
    ///
    /// - 200 on success
    /// - 500 on server failure
    ///
    /// ### 200
    ///
    /// JSON object with Info object, containing
    ///
    /// - server name: `name` as a string
    /// - server version: `version` as a string
    /// - server license: `license` as a string
    /// - server contact `contact` as a string
    /// - server users `users` an array of two ints. `[Online users, total users]`
    ///
    /// ```json
    /// {
    ///     "Info": {
    ///         "name" : String,
    ///         "version" : String,
    ///         "licesnse" : String,
    ///         "contact" : String,
    ///         "users" : [Int,Int]
    ///     }
    /// }
    /// ```
    ///
    /// ### 500
    ///
    /// ### 404, 400, 401, 500
    ///
    /// ```
    /// {
    ///     "Error": String
    /// }
    /// ```
    ///
    fn info(state: Self::SelfLock, req: Request) -> impl Future<Output = Result<Response>>;

    /// # Login
    ///
    /// Login user, setting them online and an optional status.
    /// Users must be set offline automatically after 1 hour.
    ///
    /// ## Endpoints
    ///
    /// Must be avilable at `/login`
    ///
    /// ## Headers
    ///
    /// A request header `Authorization` must be provided with user's password.
    ///
    /// ## Parameters
    ///
    /// - `username` must be provided with user's username
    /// - `status` can be provided with a string to set. Spaces in text represented with `+.`.
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
    ///
    /// ### 404, 400, 401, 500
    ///
    /// ```
    /// {
    ///     "Error": String
    /// }
    /// ```
    fn login(state: Self::SelfLock, req: Request) -> impl Future<Output = Result<Response>>;

    /// # Logoff
    ///
    /// Logoff user, setting them offline
    ///
    /// ## Endpoints
    ///
    /// Must be avilable at `/logoff`
    ///
    /// ## Headers
    ///
    /// - `Authorization` must be provided with the user's password.
    ///
    /// ## Parameters
    ///
    /// - `username` must be provided with user's username
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
    ///
    /// ### 404, 400, 401, 500
    ///
    /// ```
    /// {
    ///     "Error": String
    /// }
    /// ```
    fn logoff(state: Self::SelfLock, req: Request) -> impl Future<Output = Result<Response>>;

    /// # Snuggle
    ///
    /// Snuggle a user
    ///
    /// ## Endpoints
    ///
    /// Must be avilable at `/snuggle`
    ///
    /// ## Headers
    ///
    /// A request header `Authorization` must be provided with the requesting user's password.
    ///
    /// ## Parameters
    ///
    /// - username: username of user doing the snuggling
    /// - user: username, plain or federated, of the user to be snuggled.
    ///
    /// > [!NOTE]
    /// > Username: plain username `pockets` for server users.
    /// > Federated username: username with a server name `pockets@pockets.buzz` to check local or another server.
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
    /// Replies with JSON object `User`.
    ///
    /// ```
    /// {
    ///     "User" : {
    ///         "username" : "pockets@localhost"
    ///         "website" : null/String,
    ///         "socials" : {
    ///             "website": "https://example.com"
    ///         } ,
    ///         "bio" : null/String
    ///     }
    /// }
    /// ```
    fn snuggle(state: Self::SelfLock, req: Request) -> impl Future<Output = Result<Response>>;

    /// # Check
    ///
    /// Check users who snuggled
    ///
    /// ## Endpoints
    ///
    /// Endpoint must be available at `/check`.
    ///
    /// ## Headers
    ///
    /// - `Authorization` for current user's password.
    ///
    /// ## Parameters
    ///
    /// - `username` for current user's username.
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
    /// JSON object `List` with a list of `User`.
    /// ```
    /// {
    ///     "List": [
    ///         {
    ///             "User": {
    ///                 "username": String,
    ///                 "website": null/String,
    ///                 "socials": {
    ///                     "website": String
    ///                 },
    ///             "bio": null
    ///             }
    ///         }
    ///     ]
    /// }
    /// ```
    ///
    fn check(state: Self::SelfLock, req: Request) -> impl Future<Output = Result<Response>>;

    /// # Bump
    ///
    /// Bump's a user to keep them online for longer than the standard hour.
    ///
    ///
    /// ## Endpoints
    ///
    /// Must be avilable at `/bump`
    ///
    /// ## Headers
    ///
    /// - `Authorization` must be provided with user's password.
    ///
    /// ## Parameters
    ///
    /// - `username` must be provided.
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
    ///
    /// ### 404, 400, 401, 500
    ///
    /// ```
    /// {
    ///     "Error": String
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
    /// JSON object array with `User` objects
    ///
    /// ```
    /// {
    ///     "List": [{
    ///         "User": {
    ///             "username": "kiwi@localhost",
    ///             "website":null,
    ///             "socials": {},
    ///             "bio": null
    ///         }
    ///     },
    ///     {
    ///         "User": {
    ///             "username":
    ///             "pockets@localhost",
    ///             "website": null,
    ///             "socials": {
    ///                 "github": "https://github.com/pocketspocketspockets"
    ///             },
    ///             "bio": null
    ///         }
    ///     }]
    /// }
    /// ```
    fn list(state: Self::SelfLock, req: Request) -> impl Future<Output = Result<Response>>;

    /// # Register
    ///
    /// Register a new user account on the current server.
    ///
    /// ## Endpoints
    ///
    /// Must be avilable at `/register`
    ///
    /// ## Parameters
    ///
    /// - `key` optional server registration key
    /// - `username` must be provided with new username
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
    ///
    /// ### 404, 400, 401, 500
    ///
    /// ```
    /// {
    ///     "Error": String
    /// }
    /// ```
    fn register(state: Self::SelfLock, req: Request) -> impl Future<Output = Result<Response>>;

    /// # Deregister
    ///
    /// Register a new user account on the current server.
    ///
    /// ## Endpoints
    ///
    /// Must be avilable at `/deregister`
    ///
    /// ## Headers
    ///
    /// - `Authorization` required for current user
    ///
    /// ## Parameters
    ///
    /// - `key` optional server registration key
    /// - `username` must be provided with new username
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
    ///
    /// ### 404, 400, 401, 500
    ///
    /// ```
    /// {
    ///     "Error": String
    /// }
    /// ```
    fn deregister(state: Self::SelfLock, req: Request) -> impl Future<Output = Result<Response>>;

    /// # set bio
    /// 
    /// Set a user's biography, or description
    /// 
    /// ## Endpoint
    /// 
    /// `/setbio` required.
    /// 
    /// ## Headers
    /// 
    /// - `Authorization` required with user's password
    /// 
    /// ## Parameters
    /// 
    /// - `username` requried with user's username
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
    ///
    /// ### 404, 400, 401, 500
    ///
    /// ```
    /// {
    ///     "Error": String
    /// }
    /// ```
    fn set_bio(state: Self::SelfLock, req: Request) -> impl Future<Output = Result<Response>>;

    /// # add social
    /// 
    /// Adds a social link to a user's profile
    /// 
    /// ## Endpoint
    /// 
    /// `/addsocial` required.
    /// 
    /// ## Headers
    /// 
    /// - `Authorization` required with user's password
    /// 
    /// ## Parameters
    /// 
    /// - `username` requried with user's username
    /// - `name` name of service.
    /// - `string` url of service.
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
    ///
    /// ### 404, 400, 401, 500
    ///
    /// ```
    /// {
    ///     "Error": String
    /// }
    /// ```
    fn add_social(state: Self::SelfLock, req: Request) -> impl Future<Output = Result<Response>>;

    /// # remove social
    /// 
    /// Adds a social link to a user's profile
    /// 
    /// ## Endpoint
    /// 
    /// `/delsocial`
    /// 
    /// ## Headers
    /// 
    /// - `Authorization` required with user's password
    /// 
    /// ## Parameters
    /// 
    /// - `username` requried with user's username
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
    ///
    /// ### 404, 400, 401, 500
    ///
    /// ```
    /// {
    ///     "Error": String
    /// }
    /// ```
    fn remove_social(state: Self::SelfLock, req: Request)
    -> impl Future<Output = Result<Response>>;

    /// # add social
    /// 
    /// Adds a social link to a user's profile
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
    ///
    /// ### 404, 400, 401, 500
    ///
    /// ```
    /// {
    ///     "Error": String
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
    /// TODO: Finish Responses
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
    /// TODO: Responses
    fn fingerprint(state: Self::SelfLock, req: Request) -> impl Future<Output = Result<Response>>;

    // fn save_users(state: Self::SelfLock, req: Request) -> impl Future<Output = Result<()>>;
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
