//! Streaming download helpers: hashing, PE header check and progress.

use crate::error;
use lightproxy_core::CoreError;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

pub fn sha256_bytes(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

pub fn sha256_file(path: &Path) -> Result<String, CoreError> {
    let file = File::open(path).map_err(|_| error::download_failed("读取安装包失败"))?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|_| error::download_failed("读取安装包失败"))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

pub fn has_mz(path: &Path) -> bool {
    let Ok(mut file) = File::open(path) else {
        return false;
    };
    let mut header = [0u8; 2];
    file.read_exact(&mut header).is_ok() && &header == b"MZ"
}

pub fn parse_sha256(body: &str) -> Option<String> {
    let token = body.split_whitespace().next()?;
    if token.len() == 64 && token.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Some(token.to_ascii_lowercase())
    } else {
        None
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Progress {
    pub received: u64,
    pub total: Option<u64>,
    pub source: String,
}
