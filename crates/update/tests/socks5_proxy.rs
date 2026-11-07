mod common;

use lightproxy_update::{sha256_bytes, Source, UpdateClient};
use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

const VERSION: &str = "0.9.0";

fn asset(version: &str) -> String {
    format!("/download/v{version}/LightProxy_{version}_x64-setup.exe")
}

fn sha_asset(version: &str) -> String {
    format!("{}.sha256", asset(version))
}

/// 最小 SOCKS5 中转：无认证握手，按请求端口连接 127.0.0.1 并双向转发。
fn start_socks5_relay() -> (SocketAddr, Arc<AtomicUsize>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("绑定 SOCKS5 夹具失败");
    let addr = listener.local_addr().unwrap();
    let connections = Arc::new(AtomicUsize::new(0));
    let recorded = connections.clone();
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut client) = stream else {
                break;
            };
            recorded.fetch_add(1, Ordering::SeqCst);
            thread::spawn(move || {
                let mut greeting = [0u8; 3];
                if client.read_exact(&mut greeting).is_err() || greeting != [0x05, 0x01, 0x00] {
                    return;
                }
                if client.write_all(&[0x05, 0x00]).is_err() {
                    return;
                }
                let mut head = [0u8; 4];
                if client.read_exact(&mut head).is_err() || head[0] != 0x05 || head[1] != 0x01 {
                    return;
                }
                let port = match head[3] {
                    0x01 => {
                        let mut rest = [0u8; 6];
                        if client.read_exact(&mut rest).is_err() {
                            return;
                        }
                        u16::from_be_bytes([rest[4], rest[5]])
                    }
                    0x03 => {
                        let mut len = [0u8; 1];
                        if client.read_exact(&mut len).is_err() {
                            return;
                        }
                        let mut rest = vec![0u8; len[0] as usize + 2];
                        if client.read_exact(&mut rest).is_err() {
                            return;
                        }
                        u16::from_be_bytes([rest[rest.len() - 2], rest[rest.len() - 1]])
                    }
                    _ => return,
                };
                let Ok(mut upstream) = TcpStream::connect(("127.0.0.1", port)) else {
                    let _ = client.write_all(&[0x05, 0x05, 0x00, 0x01, 0, 0, 0, 0, 0, 0]);
                    return;
                };
                if client
                    .write_all(&[0x05, 0x00, 0x00, 0x01, 127, 0, 0, 1, 0, 0])
                    .is_err()
                {
                    return;
                }
                let mut client_reader = match client.try_clone() {
                    Ok(stream) => stream,
                    Err(_) => return,
                };
                let mut upstream_writer = match upstream.try_clone() {
                    Ok(stream) => stream,
                    Err(_) => return,
                };
                thread::spawn(move || {
                    let _ = std::io::copy(&mut client_reader, &mut upstream_writer);
                });
                let _ = std::io::copy(&mut upstream, &mut client);
            });
        }
    });
    (addr, connections)
}

fn proxy_sources(base: &str) -> Vec<Source> {
    let mut source = Source::local("代理源", base);
    source.via_proxy = true;
    vec![source]
}

const FEED: &str = r#"<feed>
    <entry><link rel="alternate" href="https://github.com/BeafJerky/LightProxy/releases/tag/v0.9.0"/></entry>
</feed>"#;

#[test]
fn check_reaches_a_local_source_through_socks5() {
    let server = common::start(vec![(
        "/releases.atom".into(),
        200,
        FEED.as_bytes().to_vec(),
    )]);
    let (proxy, connections) = start_socks5_relay();
    let proxy_url = format!("socks5h://{proxy}");
    let client = UpdateClient::new("0.8.0", Some(&proxy_url)).unwrap();

    let outcome = client
        .check(&proxy_sources(&common::base(&server)))
        .unwrap();

    assert_eq!(outcome.latest_version, "0.9.0");
    assert_eq!(outcome.source, "代理源");
    assert!(
        connections.load(Ordering::SeqCst) > 0,
        "SOCKS5 夹具未被使用"
    );
}

#[test]
fn download_reaches_a_local_source_through_socks5() {
    let body = common::package_bytes();
    let hash = sha256_bytes(&body);
    let server = common::start(vec![
        (asset(VERSION), 200, body.clone()),
        (
            sha_asset(VERSION),
            200,
            format!("{hash}  LightProxy_{VERSION}_x64-setup.exe\n").into_bytes(),
        ),
    ]);
    let (proxy, connections) = start_socks5_relay();
    let proxy_url = format!("socks5h://{proxy}");
    let client = UpdateClient::new("0.8.0", Some(&proxy_url)).unwrap();
    let dir = tempfile::tempdir().unwrap();

    let package = client
        .download(
            VERSION,
            &proxy_sources(&common::base(&server)),
            dir.path(),
            &mut |_| {},
        )
        .unwrap();

    assert_eq!(package.sha256, hash);
    assert_eq!(package.source, "代理源");
    assert_eq!(
        fs::metadata(dir.path().join(&package.file_name))
            .unwrap()
            .len(),
        body.len() as u64
    );
    assert!(
        connections.load(Ordering::SeqCst) > 0,
        "SOCKS5 夹具未被使用"
    );
}
