pub mod networking;
pub mod prelude;
pub mod userlist;

use self::networking::{Request, Response};
use self::prelude::*;

pub trait Fngr {
    type SelfLock;

    fn info(state: Self::SelfLock, req: Request) -> impl Future<Output = Result<Response>>;
    fn login(state: Self::SelfLock, req: Request) -> impl Future<Output = Result<Response>>;
    fn logoff(state: Self::SelfLock, req: Request) -> impl Future<Output = Result<Response>>;
    fn finger(state: Self::SelfLock, req: Request) -> impl Future<Output = Result<Response>>;
    fn check(state: Self::SelfLock, req: Request) -> impl Future<Output = Result<Response>>;
    fn bump(state: Self::SelfLock, req: Request) -> impl Future<Output = Result<Response>>;
    fn list(state: Self::SelfLock, req: Request) -> impl Future<Output = Result<Response>>;
    fn register(state: Self::SelfLock, req: Request) -> impl Future<Output = Result<Response>>;
    fn deregister(state: Self::SelfLock, req: Request) -> impl Future<Output = Result<Response>>;
    fn set_bio(state: Self::SelfLock, req: Request) -> impl Future<Output = Result<Response>>;
    fn add_social(state: Self::SelfLock, req: Request) -> impl Future<Output = Result<Response>>;
    fn remove_social(state: Self::SelfLock, req: Request)
    -> impl Future<Output = Result<Response>>;
    fn set_website(state: Self::SelfLock, req: Request) -> impl Future<Output = Result<Response>>;
}

#[cfg(feature = "blocking")]
mod blocking {
    use super::prelude::*;
    use crate::networking::{Request, Response};

    pub trait Fngr {
        fn info(&mut self, req: Request) -> Result<Response>;
        fn login(&mut self, req: Request) -> Result<Response>;
        fn logoff(&mut self, req: Request) -> Result<Response>;
        fn finger(&mut self, req: Request) -> Result<Response>;
        fn check(&mut self, req: Request) -> Result<Response>;
        fn bump(&mut self, req: Request) -> Result<Response>;
        fn list(&mut self, req: Request) -> Result<Response>;
        fn register(&mut self, req: Request) -> Result<Response>;
        fn deregister(&mut self, req: Request) -> Result<Response>;
        fn set_bio(&mut self, req: Request) -> Result<Response>;
        fn add_social(&mut self, req: Request) -> Result<Response>;
        fn remove_social(&mut self, req: Request) -> Result<Response>;
        fn set_website(&mut self, req: Request) -> Result<Response>;
    }
}
