pub mod networking;
pub mod prelude;
pub mod userlist;

use self::prelude::*;
use self::networking::{Request, Response};


pub trait Fngr {
    type SelfLock;

    fn login(state: Self::SelfLock, req: Request) -> impl Future<Output = Result<Response>>;
    fn logoff(state: Self::SelfLock, req: Request) -> impl Future<Output = Result<Response>>;
    fn finger(state: Self::SelfLock, req: Request) -> impl Future<Output = Result<Response>>;
    fn check(state: Self::SelfLock, req: Request) -> impl Future<Output = Result<Response>>;
    fn bump(state: Self::SelfLock, req: Request) -> impl Future<Output = Result<Response>>;
    fn list(state: Self::SelfLock, req: Request) -> impl Future<Output = Result<Response>>;
    fn register(state: Self::SelfLock, req: Request) -> impl Future<Output = Result<Response>>;
    fn deregister(state: Self::SelfLock, req: Request) -> impl Future<Output = Result<Response>>;
}
