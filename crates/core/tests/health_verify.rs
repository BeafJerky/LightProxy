use lightproxy_core::{health, CheckReport, HealthResult, Profile, Protocol};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

fn profile(protocol: Protocol, addr: SocketAddr) -> Profile {
    Profile {
        id: "p1".into(),
        name: "测试".into(),
        protocol,
        host: "127.0.0.1".into(),
        port: addr.port(),
    }
}

/// 接受一次连接，立即写出固定响应，再把读到的请求交给测试断言。
fn reply_once(reply: &'static [u8]) -> (SocketAddr, Receiver<Vec<u8>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("绑定失败");
    let addr = listener.local_addr().unwrap();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let Ok((mut stream, _)) = listener.accept() else {
            return;
        };
        stream.set_read_timeout(Some(Duration::from_secs(2))).ok();
        let _ = stream.write_all(reply);
        let _ = stream.flush();
        let mut buf = [0u8; 1024];
        let read = stream.read(&mut buf).unwrap_or(0);
        let _ = tx.send(buf[..read].to_vec());
    });
    (addr, rx)
}

fn silent_server() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").expect("绑定失败");
    let addr = listener.local_addr().unwrap();
    thread::spawn(move || {
        if let Ok((stream, _)) = listener.accept() {
            thread::sleep(Duration::from_millis(300));
            drop(stream);
        }
    });
    addr
}

/// 完成 SOCKS5 握手并捕获 CONNECT 请求；`reply` 是 10 字节成功/失败响应。
fn socks5_server(method: [u8; 2], reply: [u8; 10]) -> (SocketAddr, Receiver<Vec<u8>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("绑定失败");
    let addr = listener.local_addr().unwrap();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let Ok((mut stream, _)) = listener.accept() else {
            return;
        };
        stream.set_read_timeout(Some(Duration::from_secs(2))).ok();
        let mut greeting = [0u8; 3];
        if stream.read_exact(&mut greeting).is_err() {
            return;
        }
        if stream.write_all(&method).is_err() {
            return;
        }
        if method == [0x05, 0xFF] {
            return;
        }
        let mut head = [0u8; 4];
        if stream.read_exact(&mut head).is_err() {
            return;
        }
        let mut request = head.to_vec();
        match head[3] {
            0x01 => {
                let mut rest = [0u8; 6];
                if stream.read_exact(&mut rest).is_err() {
                    return;
                }
                request.extend_from_slice(&rest);
            }
            0x03 => {
                let mut len = [0u8; 1];
                if stream.read_exact(&mut len).is_err() {
                    return;
                }
                let mut rest = vec![0u8; len[0] as usize + 2];
                if stream.read_exact(&mut rest).is_err() {
                    return;
                }
                request.extend_from_slice(&len);
                request.extend_from_slice(&rest);
            }
            _ => return,
        }
        let _ = tx.send(request);
        let _ = stream.write_all(&reply);
    });
    (addr, rx)
}

#[test]
fn http_connect_passes_on_2xx() {
    let (addr, request) = reply_once(b"HTTP/1.1 200 Connection Established\r\n\r\n");

    let result = health::verify(&profile(Protocol::Http, addr), Duration::from_secs(2));

    assert!(result.ok, "{:?}", result.error);
    assert_eq!(result.stage, "proxy");
    let request = request.recv_timeout(Duration::from_secs(2)).unwrap();
    let request = String::from_utf8_lossy(&request);
    assert!(request.starts_with("CONNECT github.com:443 HTTP/1.1"));
}

#[test]
fn http_connect_reports_the_status_code() {
    let (addr, _) = reply_once(b"HTTP/1.1 407 Proxy Authentication Required\r\n\r\n");

    let result = health::verify(&profile(Protocol::Https, addr), Duration::from_secs(2));

    assert!(!result.ok);
    assert!(result.error.unwrap().contains("407"));
}

#[test]
fn http_connect_rejects_unparseable_responses() {
    let (addr, _) = reply_once(b"NOT HTTP\r\n\r\n");

    let result = health::verify(&profile(Protocol::Http, addr), Duration::from_secs(2));

    assert!(!result.ok);
    assert_eq!(result.error.unwrap(), "代理返回了无法识别的响应");
}

#[test]
fn socks5_sends_a_domain_connect_for_github() {
    let (addr, request) = socks5_server([0x05, 0x00], [0x05, 0x00, 0x00, 0x01, 127, 0, 0, 1, 0, 0]);

    let result = health::verify(&profile(Protocol::Socks5, addr), Duration::from_secs(2));

    assert!(result.ok, "{:?}", result.error);
    let request = request.recv_timeout(Duration::from_secs(2)).unwrap();
    assert_eq!(&request[..4], &[0x05, 0x01, 0x00, 0x03]);
    let domain_len = request[4] as usize;
    assert_eq!(&request[5..5 + domain_len], b"github.com");
    assert_eq!(&request[5 + domain_len..], &[0x01, 0xBB]);
}

#[test]
fn socks5_reports_method_rejection_as_authentication() {
    let (addr, _) = socks5_server([0x05, 0xFF], [0; 10]);

    let result = health::verify(&profile(Protocol::Socks5, addr), Duration::from_secs(2));

    assert!(!result.ok);
    assert!(result.error.unwrap().contains("认证"));
}

#[test]
fn socks5_reports_connect_reply_failures() {
    let (addr, _) = socks5_server([0x05, 0x00], [0x05, 0x05, 0x00, 0x01, 0, 0, 0, 0, 0, 0]);

    let result = health::verify(&profile(Protocol::Socks5, addr), Duration::from_secs(2));

    assert!(!result.ok);
    assert!(result.error.unwrap().contains("拒绝"));
}

#[test]
fn verify_times_out_when_the_proxy_never_answers() {
    let result = health::verify(
        &profile(Protocol::Http, silent_server()),
        Duration::from_millis(100),
    );

    assert!(!result.ok);
    assert_eq!(result.error.unwrap(), "协议检查超时");
}

#[test]
fn verify_timeout_floor_is_two_seconds() {
    assert_eq!(health::verify_timeout(300), Duration::from_millis(2_000));
    assert_eq!(health::verify_timeout(3_000), Duration::from_millis(3_000));
}

#[test]
fn check_report_serializes_with_camel_case_fields() {
    let tcp = HealthResult {
        ok: true,
        stage: "tcp".into(),
        latency_ms: 5,
        checked_at_ms: 10,
        error: None,
    };
    let report = CheckReport { tcp, proxy: None };

    let json = serde_json::to_value(&report).unwrap();

    assert_eq!(json["tcp"]["latencyMs"], 5);
    assert_eq!(json["tcp"]["checkedAtMs"], 10);
    assert!(json["proxy"].is_null());
}
