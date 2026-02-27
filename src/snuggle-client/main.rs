use std::sync::Arc;

use reqwest::Client as Http;
use serde::{Deserialize, Serialize};
use snuggle::networking::JSONResponse;
use tokio::sync::Mutex;

type SelfLock = Arc<Mutex<Client>>;

#[derive(Serialize, Deserialize)]
struct Config {
    username: String,
    password: String,
    server: String,
}

impl Config {
    async fn load() -> anyhow::Result<Self> {
        let p = format!("{}/.snuggle", std::env::var("HOME")?);
        let file = tokio::fs::read_to_string(&p).await?;
        Ok(toml::from_str(&file)?)
    }
}

struct Client {
    http: Http,
    uesrname: String,
    // federated_username: String,
    server: String,
    passphrase: String,
}

impl Client {
    fn new(username: &str, server: &str, passphrase: &str) -> Self {
        // let federated_username = format!("{}@{}", username, server);

        Self {
            http: Http::new(),
            uesrname: username.to_owned(),
            // federated_username,
            server: server.to_owned(),
            passphrase: passphrase.to_owned(),
        }
    }

    async fn url_append(state: SelfLock, endpoint: &str) -> String {
        let lock = state.lock().await;
        format!("https://{}/{}", lock.server, endpoint)
    }

    async fn url_append_with_auth(state: SelfLock, endpoint: &str) -> String {
        let url = Self::url_append(state.clone(), endpoint).await;
        let lock = state.lock().await;
        format!(
            "{}?username={}&auth={}",
            url, lock.uesrname, lock.passphrase
        )
    }

    #[inline]
    fn info(state: SelfLock) -> impl Future<Output = String> {
        Self::url_append(state, "info")
    }

    async fn login(state: SelfLock, status: Option<String>) -> String {
        let url = Self::url_append_with_auth(state, "login").await;
        if let Some(status) = status {
            format!("{}&status={}", url, status)
        } else {
            url
        }
    }

    #[inline]
    fn logoff(state: SelfLock) -> impl Future<Output = String> {
        Self::url_append_with_auth(state, "logoff")
    }

    async fn snuggle(state: SelfLock, user: &str) -> String {
        let url = Self::url_append_with_auth(state, "snuggle").await;
        format!("{}&user={}", url, user)
    }

    fn check(state: SelfLock) -> impl Future<Output = String> {
        Self::url_append_with_auth(state, "check")
    }

    #[inline]
    fn bump(state: SelfLock) -> impl Future<Output = String> {
        Self::url_append_with_auth(state, "bump")
    }

    #[inline]
    fn list(state: SelfLock) -> impl Future<Output = String> {
        Self::url_append(state, "list")
    }

    async fn register(state: SelfLock) -> String {
        let url = Self::url_append(state.clone(), "register").await;
        let lock = state.lock().await;
        format!("{}?username={}&password={}", url, lock.uesrname, lock.passphrase)
    }

    fn deregister(state: SelfLock) -> impl Future<Output = String> {
       Self::url_append_with_auth(state, "deregister")
    }

    async fn set_bio(state: SelfLock, bio: Option<String>) -> String {
        let url = Self::url_append_with_auth(state, "setbio").await;
        match bio {
            Some(bio) => format!("{}&bio={}", url, bio),
            None => url,
        }
    }

    async fn add_social(state: SelfLock, name: String, address: String) -> String {
        let url = Self::url_append_with_auth(state, "addsocial").await;
        format!("{}&name={}&string={}", url, name, address)
    }

    async fn remove_social(state: SelfLock, name: String) -> String {
        let url = Self::url_append_with_auth(state, "delsocial").await;
        format!("{}&name={}", url, name)
    }

    async fn set_website(state: SelfLock, address: Option<String>) -> String {
        let url = Self::url_append_with_auth(state, "setweb").await;
        match address {
            Some(address) => format!("{}&addr={}", url, address),
            None => url,
        }
    }

    async fn make_request(&mut self, url: &str) -> anyhow::Result<JSONResponse> {
        let r = self.http.get(url).send().await?;
        let s = r.text().await?;
        // println!("{}", s);
        Ok(serde_json::from_str(&s)?)
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut args = pico_args::Arguments::from_env();
    let config = Config::load().await?;
    // let username: String = args.value_from_str(["-u", "--username"])?;
    // let password: String = args.value_from_str(["-p", "--password"])?;
    // let server: String = args.value_from_str(["-s", "--server"])?;

    let state = Arc::new(Mutex::new(Client::new(&config.username, &config.server, &config.password)));

    let subcmd = args.subcommand()?;

    let client = state.clone();
    let url = match subcmd {
        Some(s) => {
            match s.as_str() {
                "info" => Client::info(client).await,
                "login" => Client::login(client, args.opt_value_from_str(["-S", "--status"])?).await,
                "logoff" => Client::logoff(client).await,
                "check" => Client::check(client).await,
                "bump" => Client::bump(client).await,
                "list" => Client::list(client).await,
                "register" => Client::register(client).await,
                "deregister" => Client::deregister(client).await,
                "bio" => Client::set_bio(client, args.opt_value_from_str(["-b", "--bio"])?).await,
                "addsocial" => Client::add_social(client, args.value_from_str(["-n", "--name"])?, args.value_from_str(["-S", "--str"])?).await,
                "delsocial" => Client::remove_social(client, args.value_from_str(["-n", "--name"])?).await,
                "web" => Client::set_website(client, args.opt_value_from_str(["-w", "--website"])?).await,
                _ => Client::snuggle(client, &s).await,
            }
        },
        None => return Ok(()),
    };

    let resp = state.lock().await.make_request(&url).await?;

    match &resp {
        JSONResponse::Error(e) => eprintln!("error: {}", e),
        JSONResponse::User { username, status, website, socials, bio } => {
            println!("{}\t\"{}\"\n{}\n{} since {} seconds ago\nwebsite:\t{}\n{:#?}",
            username,
            status.text.clone().unwrap_or("".to_owned()),
            bio.clone().unwrap_or("".to_owned()),
            if status.online { "online".to_owned() } else { "offline".to_owned() },
            status.since,
            website.clone().unwrap_or("".to_owned()),
            socials)
        },
        JSONResponse::List(jsonresponses) => {
            eprintln!("unimplemented\n{:#?}", jsonresponses)
        },
        JSONResponse::Ok(_) => {},
        JSONResponse::Log(items) => {
            eprintln!("unimplemented\n{:#?}", items)
        },
        JSONResponse::Info { name, version, licesnse, contact, users } => {
            eprintln!("unimplemented\n{:#?}", resp)
        },
    }
    Ok(())
}

#[tokio::test]
async fn test_info() {
    let client = Client::new("foo", "example.com", "bar");
    let state = Arc::new(Mutex::new(client));
    assert_eq!("https://example.com/info", Client::info(state).await);
}

#[tokio::test]
async fn test_login() {
    let client = Client::new("foo", "example.com", "bar");
    let state = Arc::new(Mutex::new(client));
    assert_eq!("https://example.com/login?username=foo&auth=bar", Client::login(state, None).await);
}

#[tokio::test]
async fn test_login_status() {
    let client = Client::new("foo", "example.com", "bar");
    let state = Arc::new(Mutex::new(client));
    assert_eq!("https://example.com/login?username=foo&auth=bar&status=hello", Client::login(state, Some("hello".to_owned())).await);
}

#[tokio::test]
async fn test_logoff() {
    let client = Client::new("foo", "example.com", "bar");
    let state = Arc::new(Mutex::new(client));
    assert_eq!("https://example.com/logoff?username=foo&auth=bar", Client::logoff(state).await);
}

#[tokio::test]
async fn test_snuggle() {
    let client = Client::new("foo", "example.com", "bar");
    let state = Arc::new(Mutex::new(client));
    assert_eq!("https://example.com/snuggle?username=foo&auth=bar&user=foo@example.com", Client::snuggle(state, "foo@example.com").await);
}

#[tokio::test]
async fn test_check() {
    let client = Client::new("foo", "example.com", "bar");
    let state = Arc::new(Mutex::new(client));
    assert_eq!("https://example.com/check?username=foo&auth=bar", Client::check(state).await);
}

#[tokio::test]
async fn test_bump() {
    let client = Client::new("foo", "example.com", "bar");
    let state = Arc::new(Mutex::new(client));
    assert_eq!("https://example.com/bump?username=foo&auth=bar", Client::bump(state).await);
}

#[tokio::test]
async fn test_list() {
    let client = Client::new("foo", "example.com", "bar");
    let state = Arc::new(Mutex::new(client));
    assert_eq!("https://example.com/list", Client::list(state).await);
}

#[tokio::test]
async fn test_register() {
    let client = Client::new("foo", "example.com", "bar");
    let state = Arc::new(Mutex::new(client));
    assert_eq!("https://example.com/register?username=foo&password=bar", Client::register(state).await);
}

#[tokio::test]
async fn test_deregister() {
    let client = Client::new("foo", "example.com", "bar");
    let state = Arc::new(Mutex::new(client));
    assert_eq!("https://example.com/deregister?username=foo&auth=bar", Client::deregister(state).await);
}

#[tokio::test]
async fn test_setbio() {
    let client = Client::new("foo", "example.com", "bar");
    let state = Arc::new(Mutex::new(client));
    assert_eq!("https://example.com/setbio?username=foo&auth=bar&bio=hi", Client::set_bio(state, Some("hi".to_owned())).await);
}

#[tokio::test]
async fn test_unsetbio() {
    let client = Client::new("foo", "example.com", "bar");
    let state = Arc::new(Mutex::new(client));
    assert_eq!("https://example.com/setbio?username=foo&auth=bar", Client::set_bio(state, None).await);
}

#[tokio::test]
async fn test_addsocial() {
    let client = Client::new("foo", "example.com", "bar");
    let state = Arc::new(Mutex::new(client));
    assert_eq!("https://example.com/addsocial?username=foo&auth=bar&name=foo&string=bar", Client::add_social(state, "foo".to_owned(), "bar".to_owned()).await);
}

#[tokio::test]
async fn test_delsocial() {
    let client = Client::new("foo", "example.com", "bar");
    let state = Arc::new(Mutex::new(client));
    assert_eq!("https://example.com/delsocial?username=foo&auth=bar&name=foo", Client::remove_social(state, "foo".to_owned()).await);
}

#[tokio::test]
async fn test_setweb() {
    let client = Client::new("foo", "example.com", "bar");
    let state = Arc::new(Mutex::new(client));
    assert_eq!("https://example.com/setweb?username=foo&auth=bar&addr=example.com", Client::set_website(state, Some("example.com".to_owned())).await);
}

#[tokio::test]
async fn test_unsetweb() {
    let client = Client::new("foo", "example.com", "bar");
    let state = Arc::new(Mutex::new(client));
    assert_eq!("https://example.com/setweb?username=foo&auth=bar", Client::set_website(state, None).await);
}
