use crate::error::{CoreError, CoreResult};
use crate::model::{Profile, Protocol};
use std::sync::atomic::{AtomicU64, Ordering};

static ID_SEQ: AtomicU64 = AtomicU64::new(1);

pub fn new_profile_id() -> String {
    let seq = ID_SEQ.fetch_add(1, Ordering::Relaxed);
    format!("p_{}_{}", crate::model::now_ms(), seq)
}

/// Parses `http://host:port`, `host:port` or `[::1]:port`.
pub fn parse_endpoint(input: &str) -> CoreResult<(Protocol, String, u16)> {
    let raw = input.trim();
    if raw.is_empty() {
        return Err(invalid("地址不能为空"));
    }
    if raw.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err(invalid("地址中不能包含空白或控制字符"));
    }
    let (protocol, rest) = match raw.split_once("://") {
        Some((scheme, rest)) => {
            let protocol = Protocol::parse(scheme).ok_or_else(|| {
                CoreError::new(
                    "invalid_protocol",
                    "validate",
                    format!("不支持的协议：{scheme}"),
                )
                .with_hint("仅支持 http、https、socks5")
            })?;
            (protocol, rest)
        }
        None => (Protocol::Http, raw),
    };
    let rest = rest.trim_end_matches('/');
    if rest.is_empty() {
        return Err(invalid("缺少主机与端口"));
    }
    let (host, port) = split_host_port(rest)?;
    let host = normalize_host(host)?;
    Ok((protocol, host, port))
}

fn split_host_port(rest: &str) -> CoreResult<(&str, u16)> {
    if let Some(stripped) = rest.strip_prefix('[') {
        let (host, tail) = stripped
            .split_once(']')
            .ok_or_else(|| invalid("IPv6 地址缺少右括号"))?;
        let port = tail
            .strip_prefix(':')
            .ok_or_else(|| invalid("地址缺少端口"))?;
        return Ok((host, parse_port(port)?));
    }
    let (host, port) = rest
        .rsplit_once(':')
        .ok_or_else(|| invalid("地址缺少端口，应为 host:port"))?;
    if host.contains(':') {
        return Err(invalid("IPv6 地址需要用方括号包裹，例如 [::1]:7890"));
    }
    Ok((host, parse_port(port)?))
}

fn parse_port(value: &str) -> CoreResult<u16> {
    let port: u16 = value
        .parse()
        .map_err(|_| invalid("端口必须是 1–65535 的整数"))?;
    if port == 0 {
        return Err(invalid("端口范围是 1–65535"));
    }
    Ok(port)
}

fn normalize_host(value: &str) -> CoreResult<String> {
    let host = value.trim().trim_end_matches('.');
    if host.is_empty() {
        return Err(invalid("主机不能为空"));
    }
    if host.len() > 253 {
        return Err(invalid("主机长度超过 253 字节"));
    }
    if !host
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | ':'))
    {
        return Err(invalid(
            "主机只允许字母、数字、点、减号、下划线与 IPv6 冒号",
        ));
    }
    if host.starts_with('-') || host.ends_with('-') {
        return Err(invalid("主机不能以减号开头或结尾"));
    }
    Ok(host.to_string())
}

pub fn validate_profile(profile: &Profile, existing: &[Profile]) -> CoreResult<()> {
    if profile.id.trim().is_empty() {
        return Err(invalid("配置 ID 不能为空"));
    }
    let name = profile.name.trim();
    if name.is_empty() {
        return Err(invalid("配置名称不能为空"));
    }
    if name.chars().count() > 40 {
        return Err(invalid("配置名称不能超过 40 个字符"));
    }
    if profile.host.trim().is_empty() {
        return Err(invalid("主机不能为空"));
    }
    if profile.port == 0 {
        return Err(invalid("端口范围是 1–65535"));
    }
    normalize_host(&profile.host)?;
    if existing
        .iter()
        .any(|other| other.id != profile.id && other.name.eq_ignore_ascii_case(name))
    {
        return Err(
            CoreError::new("duplicate_name", "validate", "配置名称已存在")
                .with_hint("换一个名称后再保存"),
        );
    }
    Ok(())
}

pub fn profile_from_input(
    id: String,
    name: String,
    protocol: String,
    host: String,
    port: u16,
) -> CoreResult<Profile> {
    let protocol = Protocol::parse(&protocol).ok_or_else(|| {
        CoreError::new("invalid_protocol", "validate", "协议必须是 http 或 https")
    })?;
    if port == 0 {
        return Err(invalid("端口范围是 1–65535"));
    }
    Ok(Profile {
        id,
        name: name.trim().to_string(),
        protocol,
        host: normalize_host(&host)?,
        port,
    })
}

fn invalid(message: impl Into<String>) -> CoreError {
    CoreError::new("invalid_endpoint", "validate", message)
}
