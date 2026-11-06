use crate::model::{now_ms, HealthResult};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

/// L1 check: the proxy port accepts a TCP connection. It does not prove the
/// proxy can reach the internet, but it catches the common "server is gone".
pub fn probe(host: &str, port: u16, timeout: Duration) -> HealthResult {
    let started = Instant::now();
    let lookup = match (host, port).to_socket_addrs() {
        Ok(addrs) => addrs.collect::<Vec<_>>(),
        Err(err) => {
            return HealthResult {
                ok: false,
                stage: "tcp".into(),
                latency_ms: started.elapsed().as_millis() as u64,
                checked_at_ms: now_ms(),
                error: Some(format!("无法解析主机：{err}")),
            }
        }
    };
    if lookup.is_empty() {
        return HealthResult {
            ok: false,
            stage: "tcp".into(),
            latency_ms: started.elapsed().as_millis() as u64,
            checked_at_ms: now_ms(),
            error: Some("主机没有可用地址".into()),
        };
    }
    let mut last_error = String::from("连接超时");
    for addr in lookup {
        match TcpStream::connect_timeout(&addr, timeout) {
            Ok(_) => {
                return HealthResult {
                    ok: true,
                    stage: "tcp".into(),
                    latency_ms: started.elapsed().as_millis() as u64,
                    checked_at_ms: now_ms(),
                    error: None,
                }
            }
            Err(err) => last_error = err.to_string(),
        }
    }
    HealthResult {
        ok: false,
        stage: "tcp".into(),
        latency_ms: started.elapsed().as_millis() as u64,
        checked_at_ms: now_ms(),
        error: Some(format!("连接失败：{last_error}")),
    }
}
