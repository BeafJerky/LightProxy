use crate::error;
use lightproxy_core::CoreError;
use std::cmp::Ordering;

pub fn is_version(value: &str) -> bool {
    let mut parts = value.split('.');
    let (Some(major), Some(minor), Some(patch), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return false;
    };
    [major, minor, patch]
        .iter()
        .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
}

pub fn parse_version(value: &str) -> Result<(u64, u64, u64), CoreError> {
    if !is_version(value) {
        return Err(error::invalid_version(value));
    }
    let mut parts = value
        .split('.')
        .map(|part| part.parse::<u64>().unwrap_or(0));
    Ok((
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
    ))
}

pub fn compare_versions(left: &str, right: &str) -> Ordering {
    match (parse_version(left), parse_version(right)) {
        (Ok(left), Ok(right)) => left.cmp(&right),
        _ => Ordering::Equal,
    }
}

pub fn is_newer(candidate: &str, current: &str) -> bool {
    is_version(candidate)
        && is_version(current)
        && compare_versions(candidate, current) == Ordering::Greater
}

pub fn asset_name(version: &str) -> String {
    format!("LightProxy_{version}_x64-setup.exe")
}

pub fn sha_asset_name(version: &str) -> String {
    format!("{}.sha256", asset_name(version))
}

pub fn release_url(version: &str) -> String {
    format!("{}/releases/tag/v{version}", crate::source::REPO_ROOT)
}
