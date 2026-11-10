use crate::health;
use crate::model::{DiagnoseReport, PathResult, PathStage, Profile, Recommendation};
use native_tls::TlsConnector;
use std::io::{Read, Write};
use std::net::{IpAddr, TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

pub const DIAGNOSE_TARGET: (&str, u16) = ("github.com", 443);

const HEADER_LIMIT: usize = 8 * 1024;

/// Runs the three transports LightProxy actually uses, serially.
pub fn diagnose(profile: &Profile, timeout: Duration) -> DiagnoseReport {
    diagnose_target(profile, DIAGNOSE_TARGET, timeout)
}

fn diagnose_target(profile: &Profile, target: (&str, u16), timeout: Duration) -> DiagnoseReport {
    let http = http_path(profile, target, timeout);
    let socks5_domain = socks5_domain_path(profile, target, timeout);
    let system_socks = system_socks_path(profile, target, timeout);
    let recommendation = recommend(&http, &socks5_domain, &system_socks);
    let summary = summarize(recommendation, &http, &socks5_domain, &system_socks);
    DiagnoseReport {
        http,
        socks5_domain,
        system_socks,
        recommendation,
        summary,
    }
}

pub fn recommend(
    http: &PathResult,
    socks5_domain: &PathResult,
    system_socks: &PathResult,
) -> Recommendation {
    if http.ok {
        Recommendation::Http
    } else if system_socks.ok {
        Recommendation::Socks5System
    } else if socks5_domain.ok {
        Recommendation::Socks5EnvOnly
    } else {
        Recommendation::Unavailable
    }
}

pub fn summarize(
    recommendation: Recommendation,
    http: &PathResult,
    socks5_domain: &PathResult,
    system_socks: &PathResult,
) -> String {
    match recommendation {
        Recommendation::Http => "HTTP 通道可用，建议使用 HTTP。".into(),
        Recommendation::Socks5System => {
            "HTTP 通道不可用，系统 SOCKS 可用，建议使用 SOCKS5。".into()
        }
        Recommendation::Socks5EnvOnly => {
            "只有 SOCKS5 远程解析可用；系统代理会按本机解析，建议仅启用环境变量同步。".into()
        }
        Recommendation::Unavailable => {
            if [http, socks5_domain, system_socks]
                .iter()
                .all(|result| result.stage == PathStage::Proxy)
            {
                "代理端口不可达，请检查地址与端口。".into()
            } else {
                "代理端口可达，但转发或解析失败，请检查代理上游。".into()
            }
        }
    }
}

fn http_path(profile: &Profile, target: (&str, u16), timeout: Duration) -> PathResult {
    let started = Instant::now();
    let mut stream = match connect_proxy(profile, timeout) {
        Ok(stream) => stream,
        Err(error) => return failed(started, PathStage::Proxy, error),
    };
    prepare(&stream, timeout);
    if let Err(error) = health::http_connect(&mut stream, target.0, target.1) {
        return failed(started, PathStage::Tunnel, tunnel_message(error));
    }
    finish(started, stream, target)
}

fn socks5_domain_path(profile: &Profile, target: (&str, u16), timeout: Duration) -> PathResult {
    let started = Instant::now();
    let mut stream = match connect_proxy(profile, timeout) {
        Ok(stream) => stream,
        Err(error) => return failed(started, PathStage::Proxy, error),
    };
    prepare(&stream, timeout);
    if let Err(error) = health::socks5_connect(&mut stream, target.0, target.1) {
        return failed(started, PathStage::Tunnel, tunnel_message(error));
    }
    finish(started, stream, target)
}

fn system_socks_path(profile: &Profile, target: (&str, u16), timeout: Duration) -> PathResult {
    let started = Instant::now();
    let mut stream = match connect_proxy(profile, timeout) {
        Ok(stream) => stream,
        Err(error) => return failed(started, PathStage::Proxy, error),
    };
    prepare(&stream, timeout);
    if let Err(error) = socks4_connect(&mut stream, target.0, target.1) {
        return failed(started, PathStage::Tunnel, error);
    }
    finish(started, stream, target)
}

fn finish(started: Instant, stream: TcpStream, target: (&str, u16)) -> PathResult {
    let connector = match TlsConnector::new() {
        Ok(connector) => connector,
        Err(error) => return failed(started, PathStage::Tls, format!("TLS 握手失败：{error}")),
    };
    let mut tls = match connector.connect(target.0, stream) {
        Ok(tls) => tls,
        Err(error) => return failed(started, PathStage::Tls, format!("TLS 握手失败：{error}")),
    };
    match read_status(&mut tls, target.0) {
        Ok(()) => PathResult {
            ok: true,
            stage: PathStage::Http,
            latency_ms: elapsed(started),
            error: None,
        },
        Err(error) => failed(started, PathStage::Http, error),
    }
}

fn prepare(stream: &TcpStream, timeout: Duration) {
    stream.set_read_timeout(Some(timeout)).ok();
    stream.set_write_timeout(Some(timeout)).ok();
}

fn connect_proxy(profile: &Profile, timeout: Duration) -> Result<TcpStream, String> {
    let addrs = (profile.host.as_str(), profile.port)
        .to_socket_addrs()
        .map_err(|error| format!("无法解析代理主机：{error}"))?;
    let mut last = None;
    for addr in addrs {
        match TcpStream::connect_timeout(&addr, timeout) {
            Ok(stream) => return Ok(stream),
            Err(error) if is_timeout(&error) => last = Some("代理连接超时".to_string()),
            Err(error) => last = Some(format!("无法连接代理：{error}")),
        }
    }
    Err(last.unwrap_or_else(|| "代理主机没有可用地址".into()))
}

fn socks4_connect(stream: &mut TcpStream, host: &str, port: u16) -> Result<(), String> {
    let ip = resolve_ipv4(host)?;
    let mut request = Vec::with_capacity(9);
    request.extend_from_slice(&[0x04, 0x01]);
    request.extend_from_slice(&port.to_be_bytes());
    request.extend_from_slice(&ip);
    request.push(0x00);
    stream.write_all(&request).map_err(io_message)?;
    let mut reply = [0u8; 8];
    stream.read_exact(&mut reply).map_err(io_message)?;
    match reply[1] {
        0x5A => Ok(()),
        0x5B => Err("代理拒绝转发 SOCKS4 请求".into()),
        _ => Err("代理返回了无法识别的 SOCKS4 响应".into()),
    }
}

fn resolve_ipv4(host: &str) -> Result<[u8; 4], String> {
    let addrs = (host, 443)
        .to_socket_addrs()
        .map_err(|error| format!("本机解析失败：{error}"))?;
    for addr in addrs {
        if let IpAddr::V4(ip) = addr.ip() {
            return Ok(ip.octets());
        }
    }
    Err("本机解析没有 IPv4 地址".into())
}

fn read_status<S: Read + Write>(stream: &mut S, host: &str) -> Result<(), String> {
    let request = format!(
        "GET / HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\nUser-Agent: LightProxy-Diagnose\r\n\r\n"
    );
    stream.write_all(request.as_bytes()).map_err(io_message)?;
    let mut response = Vec::new();
    let mut chunk = [0u8; 512];
    loop {
        let read = stream.read(&mut chunk).map_err(io_message)?;
        if read == 0 {
            return Err("目标返回了无法识别的响应".into());
        }
        response.extend_from_slice(&chunk[..read]);
        if response.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
        if response.len() > HEADER_LIMIT {
            return Err("目标返回了无法识别的响应".into());
        }
    }
    let head = String::from_utf8_lossy(&response);
    let mut parts = head.lines().next().unwrap_or_default().split_whitespace();
    let scheme = parts.next().unwrap_or_default();
    let code = parts.next().and_then(|value| value.parse::<u16>().ok());
    match code {
        Some(code) if scheme.starts_with("HTTP/1.") && (100..=599).contains(&code) => Ok(()),
        _ => Err("目标返回了无法识别的响应".into()),
    }
}

fn failed(started: Instant, stage: PathStage, error: String) -> PathResult {
    PathResult {
        ok: false,
        stage,
        latency_ms: elapsed(started),
        error: Some(error),
    }
}

fn elapsed(started: Instant) -> u64 {
    started.elapsed().as_millis() as u64
}

fn is_timeout(error: &std::io::Error) -> bool {
    matches!(
        error.kind(),
        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
    )
}

fn io_message(error: std::io::Error) -> String {
    if is_timeout(&error) {
        "等待响应超时".into()
    } else {
        format!("连接中断：{error}")
    }
}

/// `health` phrases its failures for the health-check card; the diagnosis
/// report needs transport wording instead.
fn tunnel_message(error: String) -> String {
    if error == "协议检查超时" {
        "等待响应超时".into()
    } else if let Some(detail) = error.strip_prefix("协议检查失败：") {
        format!("连接中断：{detail}")
    } else {
        error
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Profile, Protocol};
    use std::io::{Read, Write};
    use std::net::{SocketAddr, TcpListener};
    use std::sync::mpsc::{self, Receiver};
    use std::thread;
    use std::time::Duration;

    const HTTP_407: &[u8] = b"HTTP/1.1 407 Proxy Authentication Required\r\n\r\n";
    const HTTP_200: &[u8] = b"HTTP/1.1 200 Connection Established\r\n\r\n";
    const LOCAL_TARGET: (&str, u16) = ("127.0.0.1", 443);

    struct Behaviors {
        http: &'static [u8],
        socks5_method: [u8; 2],
        socks5_reply: [u8; 10],
        socks4_reply: [u8; 8],
    }

    impl Default for Behaviors {
        fn default() -> Self {
            Self {
                http: HTTP_407,
                socks5_method: [0x05, 0x00],
                socks5_reply: [0x05, 0x00, 0x00, 0x01, 127, 0, 0, 1, 0, 0],
                socks4_reply: [0x00, 0x5A, 0, 0, 0, 0, 0, 0],
            }
        }
    }

    fn dispatcher(behaviors: Behaviors) -> (SocketAddr, Receiver<Vec<u8>>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("绑定失败");
        let addr = listener.local_addr().unwrap();
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            for _ in 0..3 {
                let Ok((mut stream, _)) = listener.accept() else {
                    return;
                };
                stream.set_read_timeout(Some(Duration::from_secs(2))).ok();
                let mut first = [0u8; 1];
                if stream.peek(&mut first).is_err() {
                    return;
                }
                let request = match first[0] {
                    0x05 => {
                        let mut greeting = [0u8; 3];
                        if stream.read_exact(&mut greeting).is_err() {
                            return;
                        }
                        if stream.write_all(&behaviors.socks5_method).is_err() {
                            return;
                        }
                        if behaviors.socks5_method == [0x05, 0xFF] {
                            let _ = tx.send(greeting.to_vec());
                            continue;
                        }
                        let mut head = [0u8; 4];
                        if stream.read_exact(&mut head).is_err() {
                            return;
                        }
                        let mut request = head.to_vec();
                        let mut tail = match head[3] {
                            0x01 => vec![0u8; 6],
                            0x03 => {
                                let mut len = [0u8; 1];
                                if stream.read_exact(&mut len).is_err() {
                                    return;
                                }
                                request.push(len[0]);
                                vec![0u8; len[0] as usize + 2]
                            }
                            _ => return,
                        };
                        if stream.read_exact(&mut tail).is_err() {
                            return;
                        }
                        request.extend_from_slice(&tail);
                        let _ = stream.write_all(&behaviors.socks5_reply);
                        request
                    }
                    0x04 => {
                        let mut request = [0u8; 9];
                        if stream.read_exact(&mut request).is_err() {
                            return;
                        }
                        let _ = stream.write_all(&behaviors.socks4_reply);
                        request.to_vec()
                    }
                    _ => {
                        let mut request = Vec::new();
                        let mut buf = [0u8; 512];
                        loop {
                            let Ok(read) = stream.read(&mut buf) else {
                                return;
                            };
                            if read == 0 {
                                return;
                            }
                            request.extend_from_slice(&buf[..read]);
                            if request.windows(4).any(|window| window == b"\r\n\r\n") {
                                break;
                            }
                            if request.len() > 8 * 1024 {
                                return;
                            }
                        }
                        let _ = stream.write_all(behaviors.http);
                        request
                    }
                };
                let _ = tx.send(request);
            }
        });
        (addr, rx)
    }

    fn profile(addr: SocketAddr) -> Profile {
        Profile {
            id: "p1".into(),
            name: "测试".into(),
            protocol: Protocol::Http,
            host: "127.0.0.1".into(),
            port: addr.port(),
        }
    }

    fn closed_addr() -> SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0").expect("绑定失败");
        let addr = listener.local_addr().unwrap();
        drop(listener);
        addr
    }

    #[test]
    fn http_407_stops_at_tunnel() {
        let (addr, requests) = dispatcher(Behaviors::default());

        let report = diagnose_target(&profile(addr), LOCAL_TARGET, Duration::from_secs(2));

        assert!(!report.http.ok);
        assert_eq!(report.http.stage, PathStage::Tunnel);
        assert!(report.http.error.unwrap().contains("407"));
        let request = requests.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(String::from_utf8_lossy(&request).starts_with("CONNECT 127.0.0.1:443 HTTP/1.1"));
    }

    #[test]
    fn socks5_method_rejection_stops_at_tunnel() {
        let (addr, requests) = dispatcher(Behaviors {
            socks5_method: [0x05, 0xFF],
            ..Behaviors::default()
        });

        let report = diagnose_target(&profile(addr), LOCAL_TARGET, Duration::from_secs(2));

        assert_eq!(report.socks5_domain.stage, PathStage::Tunnel);
        assert!(report.socks5_domain.error.unwrap().contains("认证"));
        let _http = requests.recv_timeout(Duration::from_secs(2)).unwrap();
        let request = requests.recv_timeout(Duration::from_secs(2)).unwrap();
        assert_eq!(request, vec![0x05, 0x01, 0x00]);
    }

    #[test]
    fn socks4_rejection_stops_at_tunnel() {
        let (addr, requests) = dispatcher(Behaviors {
            socks4_reply: [0x00, 0x5B, 0, 0, 0, 0, 0, 0],
            ..Behaviors::default()
        });

        let report = diagnose_target(&profile(addr), LOCAL_TARGET, Duration::from_secs(2));

        assert_eq!(report.system_socks.stage, PathStage::Tunnel);
        assert!(report.system_socks.error.unwrap().contains("SOCKS4"));
        let _http = requests.recv_timeout(Duration::from_secs(2)).unwrap();
        let _socks5 = requests.recv_timeout(Duration::from_secs(2)).unwrap();
        let request = requests.recv_timeout(Duration::from_secs(2)).unwrap();
        assert_eq!(request, vec![0x04, 0x01, 0x01, 0xBB, 127, 0, 0, 1, 0x00]);
    }

    #[test]
    fn tunnel_success_reaches_tls() {
        let (addr, _requests) = dispatcher(Behaviors {
            http: HTTP_200,
            ..Behaviors::default()
        });

        let report = diagnose_target(&profile(addr), LOCAL_TARGET, Duration::from_secs(2));

        assert!(!report.http.ok);
        assert_eq!(report.http.stage, PathStage::Tls);
        assert!(report.http.error.unwrap().starts_with("TLS 握手失败："));
    }

    #[test]
    fn refused_port_reports_proxy_stage_for_all_paths() {
        let report = diagnose_target(
            &profile(closed_addr()),
            LOCAL_TARGET,
            Duration::from_millis(200),
        );

        for result in [report.http, report.socks5_domain, report.system_socks] {
            assert!(!result.ok);
            assert_eq!(result.stage, PathStage::Proxy);
        }
    }

    #[test]
    fn silent_proxy_times_out_in_tunnel_stage() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("绑定失败");
        let addr = listener.local_addr().unwrap();
        thread::spawn(move || {
            let mut held = Vec::new();
            for _ in 0..3 {
                if let Ok((stream, _)) = listener.accept() {
                    held.push(stream);
                }
            }
            thread::sleep(Duration::from_secs(1));
        });

        let report = diagnose_target(&profile(addr), LOCAL_TARGET, Duration::from_millis(150));

        assert_eq!(report.http.stage, PathStage::Tunnel);
        assert_eq!(report.http.error.unwrap(), "等待响应超时");
    }
}
