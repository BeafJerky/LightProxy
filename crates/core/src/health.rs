use crate::model::{now_ms, HealthResult, Profile, Protocol};
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

pub const VERIFY_TARGET: (&str, u16) = ("github.com", 443);

const VERIFY_HOST: &str = VERIFY_TARGET.0;
const VERIFY_PORT: u16 = VERIFY_TARGET.1;
const MIN_VERIFY_TIMEOUT: Duration = Duration::from_millis(2_000);
const HEADER_LIMIT: usize = 8 * 1024;

enum ConnectFailure {
    Resolve(String),
    Empty,
    Timeout(String),
    Io(String),
}

pub fn verify_timeout(preflight_ms: u64) -> Duration {
    Duration::from_millis(preflight_ms).max(MIN_VERIFY_TIMEOUT)
}

/// L1 check: the proxy port accepts a TCP connection. It does not prove the
/// proxy can reach the internet, but it catches the common "server is gone".
pub fn probe(host: &str, port: u16, timeout: Duration) -> HealthResult {
    let started = Instant::now();
    match connect(host, port, timeout) {
        Ok(_) => success(started, "tcp"),
        Err(ConnectFailure::Resolve(err)) => tcp_failure(started, format!("无法解析主机：{err}")),
        Err(ConnectFailure::Empty) => tcp_failure(started, "主机没有可用地址".into()),
        Err(ConnectFailure::Timeout(err) | ConnectFailure::Io(err)) => {
            tcp_failure(started, format!("连接失败：{err}"))
        }
    }
}

/// L2 check: prove the proxy can tunnel to `VERIFY_HOST:VERIFY_PORT`.
/// No TLS and no HTTP body: only the tunnel setup is verified.
pub fn verify(profile: &Profile, timeout: Duration) -> HealthResult {
    let started = Instant::now();
    let mut stream = match connect(&profile.host, profile.port, timeout) {
        Ok(stream) => stream,
        Err(ConnectFailure::Resolve(err)) => {
            return proxy_failure(started, format!("协议检查失败：无法解析代理主机（{err}）"))
        }
        Err(ConnectFailure::Empty) => {
            return proxy_failure(started, "协议检查失败：代理主机没有可用地址".into())
        }
        Err(ConnectFailure::Timeout(_)) => return proxy_failure(started, "协议检查超时".into()),
        Err(ConnectFailure::Io(err)) => {
            return proxy_failure(started, format!("协议检查失败：{err}"))
        }
    };
    stream.set_read_timeout(Some(timeout)).ok();
    stream.set_write_timeout(Some(timeout)).ok();
    let outcome = match profile.protocol {
        Protocol::Socks5 => socks5_connect(&mut stream, VERIFY_HOST, VERIFY_PORT),
        Protocol::Http | Protocol::Https => http_connect(&mut stream, VERIFY_HOST, VERIFY_PORT),
    };
    match outcome {
        Ok(()) => success(started, "proxy"),
        Err(message) => proxy_failure(started, message),
    }
}

fn connect(host: &str, port: u16, timeout: Duration) -> Result<TcpStream, ConnectFailure> {
    let addrs = (host, port)
        .to_socket_addrs()
        .map_err(|err| ConnectFailure::Resolve(err.to_string()))?
        .collect::<Vec<_>>();
    if addrs.is_empty() {
        return Err(ConnectFailure::Empty);
    }
    let mut last = ConnectFailure::Io("没有可用的连接结果".into());
    for addr in addrs {
        match TcpStream::connect_timeout(&addr, timeout) {
            Ok(stream) => return Ok(stream),
            Err(err) if is_timeout(&err) => last = ConnectFailure::Timeout(err.to_string()),
            Err(err) => last = ConnectFailure::Io(err.to_string()),
        }
    }
    Err(last)
}

pub(crate) fn http_connect(stream: &mut TcpStream, host: &str, port: u16) -> Result<(), String> {
    let request = format!(
        "CONNECT {host}:{port} HTTP/1.1\r\nHost: {host}:{port}\r\nProxy-Connection: Keep-Alive\r\n\r\n"
    );
    write_all(stream, request.as_bytes())?;
    let mut response = Vec::new();
    let mut chunk = [0u8; 512];
    loop {
        let read = stream.read(&mut chunk).map_err(io_message)?;
        if read == 0 {
            return Err("代理返回了无法识别的响应".into());
        }
        response.extend_from_slice(&chunk[..read]);
        if response.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
        if response.len() > HEADER_LIMIT {
            return Err("代理返回了无法识别的响应".into());
        }
    }
    let head = String::from_utf8_lossy(&response);
    let code = head
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|value| value.parse::<u16>().ok());
    match code {
        Some(code) if (200..=299).contains(&code) => Ok(()),
        Some(code) => Err(format!("代理拒绝转发（HTTP {code}）")),
        None => Err("代理返回了无法识别的响应".into()),
    }
}

pub(crate) fn socks5_connect(stream: &mut TcpStream, host: &str, port: u16) -> Result<(), String> {
    write_all(stream, &[0x05, 0x01, 0x00])?;
    let mut method = [0u8; 2];
    read_exact(stream, &mut method)?;
    if method == [0x05, 0xFF] {
        return Err("代理要求认证，当前版本不支持".into());
    }
    if method != [0x05, 0x00] {
        return Err("代理返回了无法识别的 SOCKS5 响应".into());
    }

    let host = host.as_bytes();
    let mut request = Vec::with_capacity(7 + host.len());
    request.extend_from_slice(&[0x05, 0x01, 0x00, 0x03, host.len() as u8]);
    request.extend_from_slice(host);
    request.extend_from_slice(&port.to_be_bytes());
    write_all(stream, &request)?;

    let mut head = [0u8; 4];
    read_exact(stream, &mut head)?;
    if head[0] != 0x05 {
        return Err("代理返回了无法识别的 SOCKS5 响应".into());
    }
    if head[1] != 0x00 {
        return Err(socks5_error(head[1]).into());
    }
    skip_reply_address(stream, head[3])
}

fn skip_reply_address(stream: &mut TcpStream, atyp: u8) -> Result<(), String> {
    let skip = match atyp {
        0x01 => 6,
        0x04 => 18,
        0x03 => {
            let mut len = [0u8; 1];
            read_exact(stream, &mut len)?;
            len[0] as usize + 2
        }
        _ => return Err("代理返回了无法识别的 SOCKS5 响应".into()),
    };
    let mut buf = [0u8; 18];
    read_exact(stream, &mut buf[..skip])
}

fn socks5_error(code: u8) -> &'static str {
    match code {
        0x01 => "代理转发失败",
        0x02 => "代理拒绝连接该目标",
        0x03 => "代理所在网络不可达",
        0x04 => "代理无法解析或到达目标",
        0x05 => "目标拒绝连接",
        0x06 => "代理连接目标超时",
        0x07 => "代理不支持 CONNECT",
        0x08 => "代理不支持域名目标",
        _ => "代理返回了无法识别的 SOCKS5 响应",
    }
}

fn read_exact(stream: &mut TcpStream, buf: &mut [u8]) -> Result<(), String> {
    stream.read_exact(buf).map_err(io_message)
}

fn write_all(stream: &mut TcpStream, buf: &[u8]) -> Result<(), String> {
    stream.write_all(buf).map_err(io_message)
}

fn is_timeout(err: &std::io::Error) -> bool {
    matches!(
        err.kind(),
        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
    )
}

fn io_message(err: std::io::Error) -> String {
    if is_timeout(&err) {
        "协议检查超时".into()
    } else {
        format!("协议检查失败：{err}")
    }
}

fn elapsed(started: Instant) -> u64 {
    started.elapsed().as_millis() as u64
}

fn success(started: Instant, stage: &str) -> HealthResult {
    HealthResult {
        ok: true,
        stage: stage.into(),
        latency_ms: elapsed(started),
        checked_at_ms: now_ms(),
        error: None,
    }
}

fn tcp_failure(started: Instant, error: String) -> HealthResult {
    HealthResult {
        ok: false,
        stage: "tcp".into(),
        latency_ms: elapsed(started),
        checked_at_ms: now_ms(),
        error: Some(error),
    }
}

fn proxy_failure(started: Instant, error: String) -> HealthResult {
    HealthResult {
        ok: false,
        stage: "proxy".into(),
        latency_ms: elapsed(started),
        checked_at_ms: now_ms(),
        error: Some(error),
    }
}
