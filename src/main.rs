use std::{
    collections::HashMap,
    fmt::{Display, Write},
    io::Cursor,
    str::FromStr,
    sync::Arc,
    time::{Duration, Instant},
};

static PORT: u16 = 6969;

use anyhow::anyhow;
use maplit::hashmap;
use sha2::Sha256;
use tokio::{
    io::{AsyncBufRead, AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufStream},
    net::TcpListener,
    sync::Mutex,
    time::sleep,
};
use tracing::{debug, info, subscriber};

struct Response<S: AsyncRead + Unpin> {
    status: ResponseStatus,
    headers: HashMap<String, String>,
    data: S,
}

#[derive(Clone, Debug)]
enum ResponseStatus {
    NotFound,
    Ok,
    Unauth,
    Bad,
}

impl Response<Cursor<Vec<u8>>> {
    fn from_html(status: ResponseStatus, data: impl ToString) -> Self {
        let bytes = data.to_string().into_bytes();

        let headers = hashmap! {
            "Content-Type".to_string() => "text/html".to_string(),
            "Content-Length".to_string() => bytes.len().to_string(),
        };

        Self {
            status,
            headers,
            data: Cursor::new(bytes),
        }
    }

    fn status_and_headers(&self) -> String {
        let headers = self
            .headers
            .iter()
            .map(|(k, v)| format!("{}: {}", k, v))
            .collect::<Vec<_>>()
            .join("\r\n");

        format!("HTTP/1.1 {}\r\n{headers}\r\n\r\n", self.status)
    }

    async fn write<O: AsyncWrite + Unpin>(mut self, stream: &mut O) -> anyhow::Result<()> {
        stream
            .write_all(self.status_and_headers().as_bytes())
            .await?;

        tokio::io::copy(&mut self.data, stream).await?;

        Ok(())
    }
}

impl Display for ResponseStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResponseStatus::NotFound => "404 Not Found",
            ResponseStatus::Ok => "200 OK",
            ResponseStatus::Unauth => "401 Unauthorized",
            ResponseStatus::Bad => "400 Bad Request",
        }
        .fmt(f)
    }
}

#[derive(Clone, Debug)]
struct UserInfo {
    username: String,
    status: bool,
    text: String,
    since: std::time::Instant,
    bumped: Option<std::time::Instant>,
}

impl Display for UserInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}\r\n{}\t\"{}\"\r\nsince: {}s",
            self.username,
            if self.status { "Online" } else { "Offline" },
            self.text,
            self.since.elapsed().as_secs()
        )
    }
}

#[derive(Debug)]
struct Request {
    method: Method,
    action: Action,
    name: Option<String>,
    key: Option<String>,
    user: Option<String>,
    status: Option<String>,
    headers: HashMap<String, String>,
}

impl Request {
    async fn run(
        self,
        users: Arc<Mutex<HashMap<String, (UserInfo, String)>>>,
    ) -> Response<Cursor<Vec<u8>>> {
        match self.action {
            Action::Login => self.login(users).await,
            Action::Finger => self.finger(users).await,
            Action::KeepAlive => self.up(users).await,
            Action::List => self.list(users).await,
        }
    }

    async fn list(
        self,
        users: Arc<Mutex<HashMap<String, (UserInfo, String)>>>,
    ) -> Response<Cursor<Vec<u8>>> {
        let maplock = users.lock().await;

        let mut output = String::new();

        for (username, (info, _)) in maplock.iter() {
            output
                .write_str(&format!(
                    "{}:\t{}\t\"{}\"\n",
                    username,
                    if info.status { "online" } else { "offline" },
                    info.text
                ))
                .unwrap();
        }

        Response::from_html(ResponseStatus::Ok, output)
    }

    async fn up(
        self,
        users: Arc<Mutex<HashMap<String, (UserInfo, String)>>>,
    ) -> Response<Cursor<Vec<u8>>> {
        let mut maplock = users.lock().await;

        if let Some(name) = &self.name {
            if let Some(key) = &self.key {
                if maplock[name].1 == *key && maplock[name].0.username == *name {
                    let usermut = maplock.get_mut(name).unwrap();
                    let current = usermut.0.clone();

                    usermut.0 = UserInfo {
                        username: current.username,
                        status: true,
                        text: self.status.unwrap_or(current.text),
                        since: current.since,
                        bumped: Some(Instant::now()),
                    }
                } else {
                    return Response::from_html(
                        ResponseStatus::NotFound,
                        "Invalid login key or username",
                    );
                }
            } else {
                return Response::from_html(ResponseStatus::Unauth, "login key is required");
            }
        } else {
            return Response::from_html(ResponseStatus::Unauth, "username is required");
        }

        Response::from_html(ResponseStatus::Ok, "You are now bumped")
    }

    async fn login(
        self,
        users: Arc<Mutex<HashMap<String, (UserInfo, String)>>>,
    ) -> Response<Cursor<Vec<u8>>> {
        let mut maplock = users.lock().await;

        if let Some(name) = &self.name {
            if let Some(key) = &self.key {
                if maplock[name].1 == *key && maplock[name].0.username == *name {
                    let usermut = maplock.get_mut(name).unwrap();
                    let current = usermut.0.clone();

                    usermut.0 = UserInfo {
                        username: current.username,
                        status: true,
                        text: self.status.unwrap_or(current.text),
                        since: Instant::now(),
                        bumped: None,
                    }
                } else {
                    return Response::from_html(
                        ResponseStatus::NotFound,
                        "Invalid login key or username",
                    );
                }
            } else {
                return Response::from_html(ResponseStatus::Unauth, "login key is required");
            }
        } else {
            return Response::from_html(ResponseStatus::Unauth, "username is required");
        }

        Response::from_html(ResponseStatus::Ok, "You are now online")
    }

    async fn finger(
        self,
        users: Arc<Mutex<HashMap<String, (UserInfo, String)>>>,
    ) -> Response<Cursor<Vec<u8>>> {
        let maplock = users.lock().await;

        if let Some(user) = &self.user {
            if maplock.contains_key(user) {
                Response::from_html(ResponseStatus::Ok, &maplock[user].0.to_string())
            } else {
                Response::from_html(ResponseStatus::NotFound, format!("unknown user"))
            }
        } else {
            Response::from_html(ResponseStatus::Bad, "a user to finger is required")
        }
    }
}

#[derive(Debug)]
enum Action {
    Login, // also update status
    Finger,
    KeepAlive,
    List,
}

impl FromStr for Action {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.to_lowercase();

        let r = match s.as_str() {
            "finger" => Self::Finger,
            "login" => Self::Login,
            "keepalive" | "bump" => Self::KeepAlive,
            "fingerall" | "list" => Self::List,
            a => return Err(anyhow!("invalid request: {a}")),
        };

        Ok(r)
    }
}

#[derive(Debug)]
enum Method {
    Get,
}

impl TryFrom<&str> for Method {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "GET" => Ok(Method::Get),
            m => Err(anyhow::anyhow!("unsupported method: {m}")),
        }
    }
}

async fn parse_request(mut stream: impl AsyncBufRead + Unpin) -> anyhow::Result<Request> {
    let mut line_buffer = String::new();
    stream.read_line(&mut line_buffer).await?;

    // line_buffer = line_buffer.to_lowercase();

    let mut parts = line_buffer.split_whitespace();

    let method: Method = parts
        .next()
        .ok_or(anyhow!("missing request type"))
        .and_then(TryInto::try_into)?;

    let path: String = parts.next().ok_or(anyhow!("missing url")).map(Into::into)?;

    let action: Action;
    let mut name: Option<String> = None;
    let mut key: Option<String> = None;
    let mut user: Option<String> = None;
    let mut status: Option<String> = None;

    if path.starts_with("/") {
        let s: Vec<&str> = path.split("?").collect();
        action = s[0][1..].parse()?;

        if s.len() != 1 {
            let s = s[1];
            for a in s.split("&") {
                let b: Vec<&str> = a.split("=").collect();

                match b[0] {
                    "n" => name = Some(b[1].to_owned()),
                    "k" => key = Some(b[1].to_owned()),
                    "u" => user = Some(b[1].to_owned()),
                    "s" => status = Some(b[1].to_owned()),
                    c => info!(?c, "unknown parametre"),
                }
            }
        }
    } else {
        return Err(anyhow!("missing url 2"));
    }
    // }

    let mut headers = HashMap::new();

    loop {
        line_buffer.clear();
        stream.read_line(&mut line_buffer).await?;

        if line_buffer.is_empty() || line_buffer == "\n" || line_buffer == "\r\n" {
            break;
        }

        let mut comps = line_buffer.split(":");
        let key = comps.next().ok_or(anyhow!("missing header 1"))?;
        let value = comps.next().ok_or(anyhow!("missing header 2"))?.trim();
        headers.insert(key.to_string(), value.to_string());
    }

    Ok(Request {
        method,
        action,
        name,
        key,
        user,
        status,
        headers,
    })
}

fn get_users() -> HashMap<String, (UserInfo, String)> {
    let mut map = HashMap::new();
    let userlist = std::fs::read_to_string("users.list").unwrap();

    for line in userlist.lines() {
        if !line.contains(",") {
            break;
        }

        let line = line.trim();

        let a: Vec<&str> = line.split(",").collect();
        map.insert(
            a[0].to_owned(),
            (
                UserInfo {
                    username: a[0].to_owned(),
                    status: false,
                    text: "".to_owned(),
                    since: Instant::now(),
                    bumped: None,
                },
                a[1].to_owned(),
            ),
        );
    }

    map
}

async fn offline_worker(users: Arc<Mutex<HashMap<String, (UserInfo, String)>>>) {
    info!("started automatic user status offlininator");

    loop {
        sleep(Duration::from_secs(20)).await;
        info!("checking for dead users");
        let mut maplock = users.lock().await;

        for (user, (info, _)) in maplock.iter_mut() {
            if info.status && info.since.elapsed().as_secs() > 3600 {
                if let Some(bump) = info.bumped {
                    if bump.elapsed().as_secs() < 3600 {
                        continue;
                    }
                }
                info.status = false;
                info.since = Instant::now();
                info!(?user, "user automatically set offline")
            }
        }
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let users = Arc::new(Mutex::new(get_users()));

    let listener = TcpListener::bind("127.0.0.1:38273").await.unwrap();

    let userc1 = users.clone();
    tokio::spawn(async move { offline_worker(userc1).await });

    loop {
        let (stream, addr) = listener.accept().await.unwrap();
        let mut stream = BufStream::new(stream);

        let userc = users.clone();
        tokio::spawn(async move {
            info!(?addr, "incoming connection...");

            match parse_request(&mut stream).await {
                Ok(r) => {
                    info!(?r, "connection established");
                    let resp = r.run(userc.clone()).await;
                    resp.write(&mut stream).await.unwrap();
                }
                Err(e) => {
                    info!(?e, "failed request");
                    let resp = Response::from_html(ResponseStatus::Bad, e.to_string());
                    resp.write(&mut stream).await.unwrap();
                }
            }
        });
    }
}
