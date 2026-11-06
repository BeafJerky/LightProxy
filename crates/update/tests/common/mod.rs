#![allow(dead_code)]

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener};
use std::sync::{Arc, Mutex};
use std::thread;

pub type Route = (String, u16, Vec<u8>);

/// 一条带状态码、响应头与响应体的路由；`{base}` 会被替换成夹具自身的地址。
#[derive(Debug, Clone)]
pub struct Reply {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
    send_content_length: bool,
}

impl Reply {
    pub fn new(status: u16, body: impl Into<Vec<u8>>) -> Self {
        Self {
            status,
            headers: Vec::new(),
            body: body.into(),
            send_content_length: true,
        }
    }

    pub fn header(mut self, name: &str, value: impl Into<String>) -> Self {
        self.headers.push((name.to_string(), value.into()));
        self
    }

    /// 不声明 Content-Length（连接关闭即结束），用来覆盖「读取过程中截断」的分支。
    pub fn without_content_length(mut self) -> Self {
        self.send_content_length = false;
        self
    }
}

pub struct TestServer {
    pub addr: SocketAddr,
    pub hits: Arc<Mutex<Vec<String>>>,
}

pub fn start(routes: Vec<Route>) -> TestServer {
    start_replies(
        routes
            .into_iter()
            .map(|(path, status, body)| (path, Reply::new(status, body)))
            .collect(),
    )
}

pub fn start_replies(routes: Vec<(String, Reply)>) -> TestServer {
    let listener = TcpListener::bind("127.0.0.1:0").expect("绑定测试端口失败");
    let addr = listener.local_addr().expect("读取测试地址失败");
    let base = format!("http://{addr}");
    let hits = Arc::new(Mutex::new(Vec::new()));
    let recorded = hits.clone();
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { break };
            let mut buffer = [0u8; 4096];
            let read = stream.read(&mut buffer).unwrap_or(0);
            let request = String::from_utf8_lossy(&buffer[..read]).to_string();
            let path = request
                .lines()
                .next()
                .and_then(|line| line.split_whitespace().nth(1))
                .unwrap_or("/")
                .to_string();
            recorded.lock().unwrap().push(path.clone());
            let route = routes.iter().find(|(route, _)| route == &path);
            let reply = route
                .map(|(_, reply)| reply.clone())
                .unwrap_or_else(|| Reply::new(404, b"not found".to_vec()));
            let reason = match reply.status {
                200 => "OK",
                302 => "Found",
                _ => "Error",
            };
            let mut header = format!("HTTP/1.1 {} {reason}\r\n", reply.status);
            for (name, value) in &reply.headers {
                header.push_str(&format!("{name}: {}\r\n", value.replace("{base}", &base)));
            }
            if reply.send_content_length
                && !reply
                    .headers
                    .iter()
                    .any(|(name, _)| name.eq_ignore_ascii_case("content-length"))
            {
                header.push_str(&format!("Content-Length: {}\r\n", reply.body.len()));
            }
            header.push_str("Connection: close\r\n\r\n");
            let _ = stream.write_all(header.as_bytes());
            let _ = stream.write_all(&reply.body);
            let _ = stream.flush();
        }
    });
    TestServer { addr, hits }
}

pub fn base(server: &TestServer) -> String {
    format!("http://{}", server.addr)
}

pub fn package_bytes() -> Vec<u8> {
    let mut body = b"MZ".to_vec();
    body.extend(std::iter::repeat_n(7u8, 200_000));
    body
}
