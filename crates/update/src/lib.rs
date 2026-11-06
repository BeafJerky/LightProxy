//! Update discovery, download and verification for LightProxy.

mod atom;
mod download;
mod error;
mod source;
mod version;

pub use atom::{latest, parse_releases, ReleaseEntry};
pub use error::{
    busy, check_failed, download_failed, install_failed, invalid_version, no_package,
    not_available, open_failed, verify_failed,
};
pub use source::{production_sources, validate_https, ProxySource, Source};
pub use version::{
    asset_name, compare_versions, is_newer, is_version, parse_version, release_url, sha_asset_name,
};

use lightproxy_core::CoreError;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use std::time::{Duration, Instant};

pub use crate::download::{has_mz, parse_sha256, sha256_bytes, sha256_file, Progress};

/// 单次 `download` 调用的总预算（取校验和 + 所有下载源重试）。
const DOWNLOAD_BUDGET: Duration = Duration::from_secs(900);

/// 校验和与版本检查都只是一小段文本：单次请求上限 8 秒，与检查客户端的总超时一致。
/// 不能用 900 秒预算顶替，否则一个卡住的 `.sha256` 会吃掉整个预算。
const CHECKSUM_TIMEOUT: Duration = Duration::from_secs(8);

/// Atom 与 `.sha256` 只是几十 KB 的文本：超过这个量级说明对端不是我们要的东西。
const MAX_METADATA_BYTES: u64 = 512 * 1024;

/// 安装包上限：正常 NSIS 安装包 3–5 MB，512 MB 既留足余量又能挡住失控响应。
const MAX_PACKAGE_BYTES: u64 = 512 * 1024 * 1024;

/// 自定义跳转策略不自动限制跳数，这里沿用 reqwest 默认的 10 跳上限。
const MAX_REDIRECTS: usize = 10;

/// 截止时刻内一次请求可用的超时：剩余预算与单请求上限取小；预算耗尽返回 `None`。
fn clamp_timeout(remaining: Duration, limit: Duration) -> Option<Duration> {
    (!remaining.is_zero()).then(|| remaining.min(limit))
}

fn timeout_error() -> CoreError {
    crate::error::download_failed("下载超时，请稍后重试")
}

fn package_too_large() -> CoreError {
    crate::error::download_failed(format!(
        "安装包超过 {} MB 上限",
        MAX_PACKAGE_BYTES / (1024 * 1024)
    ))
}

/// 把 reqwest 的底层错误归到几个中文类别：原始错误会带 URL 与英文技术串，不能直接给用户看。
fn transport_error(err: &reqwest::Error) -> &'static str {
    if err.is_timeout() {
        "连接超时"
    } else if err.is_connect() {
        "无法连接"
    } else if err.is_decode() || err.is_body() {
        "响应读取失败"
    } else {
        "网络请求失败"
    }
}

fn status_error(status: reqwest::StatusCode) -> String {
    format!("HTTP {}", status.as_u16())
}

/// 读取一段有大小上限的文本响应（Atom / `.sha256`）；`Err` 是可直接展示的中文原因。
fn read_metadata(response: reqwest::blocking::Response) -> Result<String, String> {
    if response
        .content_length()
        .is_some_and(|length| length > MAX_METADATA_BYTES)
    {
        return Err("响应内容过大，已停止读取".to_string());
    }
    let mut body = Vec::new();
    // 服务端可以不声明 Content-Length，所以还要在读的过程中截断。
    response
        .take(MAX_METADATA_BYTES + 1)
        .read_to_end(&mut body)
        .map_err(|_| "响应读取失败".to_string())?;
    if body.len() as u64 > MAX_METADATA_BYTES {
        return Err("响应内容过大，已停止读取".to_string());
    }
    String::from_utf8(body).map_err(|_| "响应读取失败".to_string())
}

/// 发一次 GET 并把状态码/大小/编码问题都收敛成中文原因。
fn fetch_text(
    client: &reqwest::blocking::Client,
    url: &str,
    timeout: Duration,
) -> Result<String, String> {
    let response = client
        .get(url)
        .timeout(timeout)
        .send()
        .map_err(|err| transport_error(&err).to_string())?;
    let status = response.status();
    if status.is_redirection() {
        // 只允许 https 跳转的策略会停在非 https 目标上，把它当作失败而不是读取跳转页。
        return Err("跳转地址不被允许".to_string());
    }
    if !status.is_success() {
        return Err(status_error(status));
    }
    read_metadata(response)
}

#[derive(Debug)]
pub struct CheckOutcome {
    pub latest_version: String,
    pub release_url: String,
    pub asset_name: String,
    pub available: bool,
    pub source: String,
}

#[derive(Debug, Clone)]
pub struct DownloadedPackage {
    pub version: String,
    pub file_name: String,
    pub size: u64,
    pub sha256: String,
    pub source: String,
}

pub struct UpdateClient {
    current_version: String,
    check_direct: reqwest::blocking::Client,
    check_proxy: Option<reqwest::blocking::Client>,
    download_direct: reqwest::blocking::Client,
    download_proxy: Option<reqwest::blocking::Client>,
}

impl UpdateClient {
    pub fn new(current_version: impl Into<String>, proxy: Option<&str>) -> Result<Self, CoreError> {
        let current_version = current_version.into();
        let check_direct = build_client(&current_version, None, 5, 8, ClientPath::Check)?;
        let download_direct = build_client(&current_version, None, 15, 900, ClientPath::Download)?;
        let (check_proxy, download_proxy) = match proxy {
            Some(url) => (
                Some(build_client(
                    &current_version,
                    Some(url),
                    5,
                    8,
                    ClientPath::Check,
                )?),
                Some(build_client(
                    &current_version,
                    Some(url),
                    15,
                    900,
                    ClientPath::Download,
                )?),
            ),
            None => (None, None),
        };
        Ok(Self {
            current_version,
            check_direct,
            check_proxy,
            download_direct,
            download_proxy,
        })
    }

    pub fn check(&self, sources: &[Source]) -> Result<CheckOutcome, CoreError> {
        let deadline = Instant::now() + Duration::from_secs(20);
        let mut last_error = String::new();
        for source in sources {
            let Some(atom_url) = source.atom_url.as_deref() else {
                continue;
            };
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                break;
            }
            let timeout = remaining.min(CHECKSUM_TIMEOUT);
            let client = self.client(source, false)?;
            let body = match fetch_text(client, atom_url, timeout) {
                Ok(body) => body,
                Err(reason) => {
                    last_error = format!("{}：{reason}", source.label);
                    continue;
                }
            };
            let entries = parse_releases(&body);
            let Some(entry) = latest(&entries) else {
                last_error = format!("{}：未找到有效版本标签", source.label);
                continue;
            };
            return Ok(CheckOutcome {
                available: is_newer(&entry.version, &self.current_version),
                asset_name: asset_name(&entry.version),
                latest_version: entry.version.clone(),
                release_url: entry.url.clone(),
                source: source.label.clone(),
            });
        }
        Err(crate::error::check_failed(if last_error.is_empty() {
            "没有可用的更新源".to_string()
        } else {
            last_error
        }))
    }

    pub fn download(
        &self,
        version: &str,
        sources: &[Source],
        dir: &Path,
        on_progress: &mut dyn FnMut(Progress),
    ) -> Result<DownloadedPackage, CoreError> {
        self.download_with_deadline(
            version,
            sources,
            dir,
            Instant::now() + DOWNLOAD_BUDGET,
            on_progress,
        )
    }

    /// `download` 的实际实现；截止时刻可注入，便于测试预算耗尽的分支。
    fn download_with_deadline(
        &self,
        version: &str,
        sources: &[Source],
        dir: &Path,
        deadline: Instant,
        on_progress: &mut dyn FnMut(Progress),
    ) -> Result<DownloadedPackage, CoreError> {
        if !is_version(version) {
            return Err(crate::error::invalid_version(version));
        }
        fs::create_dir_all(dir).map_err(|_| crate::error::download_failed("无法创建下载目录"))?;
        let (expected, hash_source) = self.fetch_sha256(version, sources, deadline)?;
        let file_name = asset_name(version);
        let final_path = dir.join(&file_name);
        if final_path.is_file() {
            let matches = sha256_file(&final_path)
                .map(|hash| hash == expected)
                .unwrap_or(false);
            if matches && has_mz(&final_path) {
                let size = fs::metadata(&final_path)
                    .map(|meta| meta.len())
                    .unwrap_or(0);
                return Ok(DownloadedPackage {
                    version: version.to_string(),
                    file_name,
                    size,
                    sha256: expected,
                    source: format!("{hash_source}（本地缓存）"),
                });
            }
        }
        let part_path = dir.join(format!("{file_name}.part"));
        let mut last_error = String::new();
        for source in sources {
            let _ = fs::remove_file(&part_path);
            if deadline.saturating_duration_since(Instant::now()).is_zero() {
                return Err(timeout_error());
            }
            let url = source.asset_url(version, &file_name);
            match self.stream_to(source, &url, &part_path, deadline, on_progress) {
                Ok((size, total)) => {
                    if total.is_some_and(|total| total != size) {
                        last_error = format!("{}：下载不完整", source.label);
                        continue;
                    }
                    let actual = sha256_file(&part_path)?;
                    if actual != expected {
                        let _ = fs::remove_file(&part_path);
                        return Err(crate::error::verify_failed(
                            "安装包 SHA256 校验失败，已删除损坏文件",
                        ));
                    }
                    if !has_mz(&part_path) {
                        let _ = fs::remove_file(&part_path);
                        return Err(crate::error::verify_failed(
                            "安装包内容不是可执行程序，已删除损坏文件",
                        ));
                    }
                    fs::rename(&part_path, &final_path)
                        .map_err(|_| crate::error::download_failed("保存安装包失败"))?;
                    return Ok(DownloadedPackage {
                        version: version.to_string(),
                        file_name,
                        size,
                        sha256: expected,
                        source: source.label.clone(),
                    });
                }
                Err(err) => {
                    // `CoreError` 的 Display 带 code 前缀，用户可见文案只用 message。
                    last_error = format!("{}：{}", source.label, err.message);
                }
            }
        }
        let _ = fs::remove_file(&part_path);
        if Instant::now() >= deadline {
            return Err(timeout_error());
        }
        Err(crate::error::download_failed(if last_error.is_empty() {
            "没有可用的下载源".to_string()
        } else {
            last_error
        }))
    }

    fn fetch_sha256(
        &self,
        version: &str,
        sources: &[Source],
        deadline: Instant,
    ) -> Result<(String, String), CoreError> {
        let asset = sha_asset_name(version);
        for source in sources {
            let Some(timeout) = clamp_timeout(
                deadline.saturating_duration_since(Instant::now()),
                CHECKSUM_TIMEOUT,
            ) else {
                return Err(timeout_error());
            };
            let url = source.asset_url(version, &asset);
            let client = self.client(source, false)?;
            let Ok(body) = fetch_text(client, &url, timeout) else {
                continue;
            };
            if let Some(hash) = parse_sha256(&body) {
                return Ok((hash, source.label.clone()));
            }
        }
        if Instant::now() >= deadline {
            return Err(timeout_error());
        }
        Err(crate::error::download_failed("无法获取安装包校验和"))
    }

    fn stream_to(
        &self,
        source: &Source,
        url: &str,
        path: &Path,
        deadline: Instant,
        on_progress: &mut dyn FnMut(Progress),
    ) -> Result<(u64, Option<u64>), CoreError> {
        let Some(timeout) = clamp_timeout(
            deadline.saturating_duration_since(Instant::now()),
            DOWNLOAD_BUDGET,
        ) else {
            return Err(timeout_error());
        };
        let client = self.client(source, true)?;
        let mut response = client
            .get(url)
            .timeout(timeout)
            .send()
            .map_err(|err| crate::error::download_failed(transport_error(&err)))?;
        let status = response.status();
        if status.is_redirection() {
            // 只跟随 https 的策略会停在非 https 目标上：不能把跳转页当成安装包写入。
            return Err(crate::error::download_failed("跳转地址不被允许"));
        }
        if !status.is_success() {
            return Err(crate::error::download_failed(status_error(status)));
        }
        let total = response.content_length();
        if total.is_some_and(|total| total > MAX_PACKAGE_BYTES) {
            return Err(package_too_large());
        }
        let mut file =
            File::create(path).map_err(|_| crate::error::download_failed("写入安装包失败"))?;
        let mut received = 0u64;
        let mut buffer = [0u8; 64 * 1024];
        let mut last_emit = Instant::now()
            .checked_sub(Duration::from_millis(200))
            .unwrap_or_else(Instant::now);
        on_progress(Progress {
            received,
            total,
            source: source.label.clone(),
        });
        loop {
            // reqwest 的 `.timeout` 按单次 read 计时，慢速滴流的响应能一直续命；
            // 每读一块就对照截止时刻，才能真正确保整个下载不超预算。
            if Instant::now() >= deadline {
                return Err(timeout_error());
            }
            let read = response
                .read(&mut buffer)
                .map_err(|_| crate::error::download_failed("读取响应失败"))?;
            if read == 0 {
                break;
            }
            // 服务端可以不声明 Content-Length：逐块累计也要有上限，否则磁盘会被写满。
            if received + read as u64 > MAX_PACKAGE_BYTES {
                drop(file);
                let _ = fs::remove_file(path);
                return Err(package_too_large());
            }
            file.write_all(&buffer[..read])
                .map_err(|_| crate::error::download_failed("写入安装包失败"))?;
            received += read as u64;
            if last_emit.elapsed() >= Duration::from_millis(100) {
                last_emit = Instant::now();
                on_progress(Progress {
                    received,
                    total,
                    source: source.label.clone(),
                });
            }
        }
        file.flush()
            .map_err(|_| crate::error::download_failed("写入安装包失败"))?;
        on_progress(Progress {
            received,
            total,
            source: source.label.clone(),
        });
        Ok((received, total))
    }

    fn client(
        &self,
        source: &Source,
        download: bool,
    ) -> Result<&reqwest::blocking::Client, CoreError> {
        let client = if source.via_proxy {
            if download {
                self.download_proxy.as_ref()
            } else {
                self.check_proxy.as_ref()
            }
        } else if download {
            Some(&self.download_direct)
        } else {
            Some(&self.check_direct)
        };
        client.ok_or_else(|| crate::error::check_failed("代理更新源不可用"))
    }
}

/// 决定构造失败时落在哪个错误命名空间（检查路径 / 下载路径）。
#[derive(Clone, Copy)]
enum ClientPath {
    Check,
    Download,
}

impl ClientPath {
    fn error(self, message: String) -> CoreError {
        match self {
            Self::Check => crate::error::check_failed(message),
            Self::Download => crate::error::download_failed(message),
        }
    }
}

fn build_client(
    current_version: &str,
    proxy: Option<&str>,
    connect_secs: u64,
    total_secs: u64,
    path: ClientPath,
) -> Result<reqwest::blocking::Client, CoreError> {
    let mut builder = reqwest::blocking::Client::builder()
        .user_agent(format!("LightProxy/{current_version}"))
        .connect_timeout(Duration::from_secs(connect_secs))
        .timeout(Duration::from_secs(total_secs))
        .redirect(https_only_redirects());
    builder = match proxy {
        Some(url) => builder
            .proxy(reqwest::Proxy::all(url).map_err(|_| path.error("代理地址无效".to_string()))?),
        None => builder.no_proxy(),
    };
    builder
        .build()
        .map_err(|_| path.error("创建更新客户端失败".to_string()))
}

/// reqwest 的默认策略只限制跳数，不限制目标的 scheme：任何一跳都能把 https 降级成明文 http。
/// 这里只跟随 https 目标，其余停在 3xx，由调用方按「跳转被拒绝」处理；初始地址不受影响。
fn https_only_redirects() -> reqwest::redirect::Policy {
    reqwest::redirect::Policy::custom(|attempt| {
        if attempt.previous().len() >= MAX_REDIRECTS || attempt.url().scheme() != "https" {
            attempt.stop()
        } else {
            attempt.follow()
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dead_client() -> UpdateClient {
        UpdateClient::new("0.1.2", None).unwrap()
    }

    fn expired() -> Instant {
        Instant::now() - Duration::from_secs(1)
    }

    fn unreachable_source() -> Vec<Source> {
        // 端口 1 上没有监听者；预算已耗尽时不应发出任何请求。
        vec![Source::local("源1", "http://127.0.0.1:1")]
    }

    #[test]
    fn per_request_timeout_is_clamped_by_the_remaining_budget() {
        // 预算充足：按单请求上限截断，不会把整个 15 分钟给一次请求。
        assert_eq!(
            clamp_timeout(Duration::from_secs(3600), DOWNLOAD_BUDGET),
            Some(DOWNLOAD_BUDGET)
        );
        // 预算即将耗尽：剩余时间优先，下一次请求最多只等到截止时刻。
        assert_eq!(
            clamp_timeout(Duration::from_secs(30), DOWNLOAD_BUDGET),
            Some(Duration::from_secs(30))
        );
        // 预算用完：不再发起任何请求。
        assert_eq!(clamp_timeout(Duration::ZERO, DOWNLOAD_BUDGET), None);
    }

    #[test]
    fn checksum_requests_are_capped_at_eight_seconds() {
        // 校验和只有几十字节，剩余 900 秒预算也不能让它独占整个预算。
        assert_eq!(
            clamp_timeout(Duration::from_secs(3600), CHECKSUM_TIMEOUT),
            Some(Duration::from_secs(8))
        );
        assert_eq!(
            clamp_timeout(Duration::from_secs(3), CHECKSUM_TIMEOUT),
            Some(Duration::from_secs(3))
        );
    }

    #[test]
    fn download_reports_timeout_when_the_budget_is_already_spent() {
        let dir = tempfile::tempdir().unwrap();

        let error = dead_client()
            .download_with_deadline(
                "0.1.4",
                &unreachable_source(),
                dir.path(),
                expired(),
                &mut |_| {},
            )
            .unwrap_err();

        assert_eq!(error.code, "update_download_failed");
        assert_eq!(error.message, "下载超时，请稍后重试");
        assert!(
            dir.path().read_dir().unwrap().next().is_none(),
            "预算耗尽时不应写入任何文件"
        );
    }

    #[test]
    fn stream_to_reports_timeout_when_the_budget_is_already_spent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("payload.part");
        let source = Source::local("源1", "http://127.0.0.1:1");

        let error = dead_client()
            .stream_to(
                &source,
                "http://127.0.0.1:1/payload.exe",
                &path,
                expired(),
                &mut |_| {},
            )
            .unwrap_err();

        assert_eq!(error.code, "update_download_failed");
        assert_eq!(error.message, "下载超时，请稍后重试");
        assert!(!path.exists());
    }

    #[test]
    fn checksum_lookup_reports_timeout_when_the_budget_runs_out() {
        // 没有可用更新源时，最后一个源的循环结束：预算耗尽给超时，否则给缺校验和。
        let error = dead_client()
            .fetch_sha256("0.1.4", &[], expired())
            .unwrap_err();
        assert_eq!(error.message, "下载超时，请稍后重试");

        let error = dead_client()
            .fetch_sha256("0.1.4", &[], Instant::now() + DOWNLOAD_BUDGET)
            .unwrap_err();
        assert_eq!(error.message, "无法获取安装包校验和");
    }
}
