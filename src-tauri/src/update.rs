use crate::app_ctx::AppCtx;
use crate::commands::blocking;
use crate::tray;
use lightproxy_core::{now_ms, AppState, CoreError, Profile, ProxyState};
use lightproxy_update::{
    has_mz, is_newer, is_version, production_sources, release_url, sha256_file, DownloadedPackage,
    Progress, ProxySource, UpdateClient,
};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

/// 成功检查后缓存 30 分钟：面板自动检查与手动检查都不会反复打网络。
const CACHE_TTL: Duration = Duration::from_secs(30 * 60);

/// 拉起安装程序后留给前端的收尾时间，再退出自身，避免自己占着文件。
const QUIT_DELAY: Duration = Duration::from_millis(1500);

/// 分离创建且不接管控制台的标志：安装程序必须在 LightProxy 退出后继续运行。
#[cfg(windows)]
const DETACHED: u32 = 0x0000_0208;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheck {
    pub current_version: String,
    pub latest_version: Option<String>,
    pub available: bool,
    pub release_url: Option<String>,
    pub asset_name: Option<String>,
    pub source: Option<String>,
    pub checked_at_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProgress {
    pub version: String,
    pub received: u64,
    pub total: Option<u64>,
    pub percent: Option<u32>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePackage {
    pub version: String,
    pub file_name: String,
    pub size: u64,
    pub sha256: String,
    pub source: String,
}

struct CachedCheck {
    at: Instant,
    value: UpdateCheck,
}

pub struct UpdateManager {
    inner: Arc<UpdateInner>,
}

struct UpdateInner {
    download_dir: PathBuf,
    check_cache: Mutex<Option<CachedCheck>>,
    downloading: AtomicBool,
    installing: AtomicBool,
    staged: Mutex<Option<DownloadedPackage>>,
}

impl UpdateManager {
    pub fn new(download_dir: PathBuf) -> Self {
        Self {
            inner: Arc::new(UpdateInner {
                download_dir,
                check_cache: Mutex::new(None),
                downloading: AtomicBool::new(false),
                installing: AtomicBool::new(false),
                staged: Mutex::new(None),
            }),
        }
    }

    fn cached(&self) -> Option<UpdateCheck> {
        let cache = self.inner.check_cache.lock().ok()?;
        let cached = cache.as_ref()?;
        (cached.at.elapsed() < CACHE_TTL).then(|| cached.value.clone())
    }

    fn store_check(&self, value: UpdateCheck) {
        if let Ok(mut cache) = self.inner.check_cache.lock() {
            *cache = Some(CachedCheck {
                at: Instant::now(),
                value,
            });
        }
    }

    fn staged(&self) -> Option<DownloadedPackage> {
        self.inner
            .staged
            .lock()
            .ok()
            .and_then(|staged| staged.clone())
    }

    fn set_staged(&self, value: Option<DownloadedPackage>) {
        if let Ok(mut staged) = self.inner.staged.lock() {
            *staged = value;
        }
    }

    fn begin_download(&self) -> bool {
        self.inner
            .downloading
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }

    fn end_download(&self) {
        self.inner.downloading.store(false, Ordering::SeqCst);
    }

    fn begin_install(&self) -> bool {
        self.inner
            .installing
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }

    fn end_install(&self) {
        self.inner.installing.store(false, Ordering::SeqCst);
    }

    fn download_dir(&self) -> &Path {
        &self.inner.download_dir
    }
}

impl Clone for UpdateManager {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

/// 只有 HTTP/HTTPS 配置能当更新代理用；SOCKS5 没有可复用的 HTTP 代理端点。
fn active_proxy(state: &AppState) -> Option<ProxySource> {
    if !matches!(state.state, ProxyState::On | ProxyState::Degraded) {
        return None;
    }
    let profile = active_profile(state)?;
    Some(ProxySource::for_profile(
        profile.protocol,
        profile.endpoint(),
    ))
}

fn active_profile(state: &AppState) -> Option<&Profile> {
    let id = state.active_profile_id.as_deref()?;
    state.profiles.iter().find(|profile| profile.id == id)
}

fn check_dto(current: String, outcome: lightproxy_update::CheckOutcome) -> UpdateCheck {
    UpdateCheck {
        current_version: current,
        latest_version: Some(outcome.latest_version),
        available: outcome.available,
        release_url: Some(outcome.release_url),
        asset_name: Some(outcome.asset_name),
        source: Some(outcome.source),
        checked_at_ms: now_ms(),
    }
}

fn percent_of(received: u64, total: Option<u64>) -> Option<u32> {
    total
        .filter(|total| *total > 0)
        .map(|total| ((received.min(total) * 100) / total) as u32)
}

#[tauri::command]
pub async fn check_update(app: AppHandle, force: Option<bool>) -> Result<UpdateCheck, CoreError> {
    if !force.unwrap_or(false) {
        let cached = app.state::<AppCtx>().update.cached();
        if let Some(cached) = cached {
            return Ok(cached);
        }
    }
    let state = app.state::<AppCtx>().actor.get_state()?;
    let proxy = active_proxy(&state);
    let current = app.package_info().version.to_string();
    let version = current.clone();
    let outcome = blocking(move || {
        let sources = production_sources(proxy.clone())?;
        let client = UpdateClient::new(version, proxy.as_ref().map(|item| item.url.as_str()))?;
        client.check(&sources)
    })
    .await?;
    let dto = check_dto(current, outcome);
    let manager = app.state::<AppCtx>().update.clone();
    manager.store_check(dto.clone());
    // 缓存的检查结果换了版本后，之前暂存的安装包就不能再装：清掉免得装错版本。
    let stale = manager
        .staged()
        .is_some_and(|staged| Some(staged.version.as_str()) != dto.latest_version.as_deref());
    if stale {
        manager.set_staged(None);
    }
    Ok(dto)
}

#[tauri::command]
pub async fn download_update(app: AppHandle, version: String) -> Result<UpdatePackage, CoreError> {
    if !is_version(&version) {
        return Err(lightproxy_update::invalid_version(&version));
    }
    let current = app.package_info().version.to_string();
    if !is_newer(&version, &current) {
        return Err(lightproxy_update::not_available());
    }
    let latest = ensure_latest(&app).await?;
    if latest.as_deref() != Some(version.as_str()) {
        return Err(lightproxy_update::not_available());
    }
    // 先取状态再占位：读状态失败时不能留下永远为真的“下载中”标志。
    let state = app.state::<AppCtx>().actor.get_state()?;
    let proxy = active_proxy(&state);
    let manager = app.state::<AppCtx>().update.clone();
    if !manager.begin_download() {
        return Err(lightproxy_update::busy());
    }
    let download_dir = manager.download_dir().to_path_buf();
    let emit_app = app.clone();
    let emit_version = version.clone();
    let result = blocking(move || {
        let sources = production_sources(proxy.clone())?;
        let client = UpdateClient::new(current, proxy.as_ref().map(|item| item.url.as_str()))?;
        client.download(
            &version,
            &sources,
            &download_dir,
            &mut |progress: Progress| {
                let _ = emit_app.emit(
                    "update-progress",
                    UpdateProgress {
                        version: emit_version.clone(),
                        received: progress.received,
                        total: progress.total,
                        percent: percent_of(progress.received, progress.total),
                        source: progress.source,
                    },
                );
            },
        )
    })
    .await;
    manager.end_download();
    let package = result?;
    manager.set_staged(Some(package.clone()));
    Ok(UpdatePackage {
        version: package.version,
        file_name: package.file_name,
        size: package.size,
        sha256: package.sha256,
        source: package.source,
    })
}

async fn ensure_latest(app: &AppHandle) -> Result<Option<String>, CoreError> {
    let cached = app.state::<AppCtx>().update.cached();
    if let Some(cached) = cached {
        return Ok(cached.latest_version);
    }
    let check = check_update(app.clone(), Some(true)).await?;
    Ok(check.latest_version)
}

#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), CoreError> {
    let manager = app.state::<AppCtx>().update.clone();
    if !manager.begin_install() {
        return Err(lightproxy_update::busy());
    }
    let Some(package) = manager.staged() else {
        manager.end_install();
        return Err(lightproxy_update::no_package());
    };
    let download_dir = manager.download_dir().to_path_buf();
    let result = blocking(move || launch_installer(&download_dir, &package)).await;
    match result {
        Ok(()) => {
            // 安装程序起来后再退出：立刻退出会让它拿不到文件句柄，也会打断用户看到进度。
            let handle = app.clone();
            std::thread::spawn(move || {
                std::thread::sleep(QUIT_DELAY);
                tray::quit(&handle);
            });
            Ok(())
        }
        Err(error) => {
            manager.end_install();
            Err(error)
        }
    }
}

fn launch_installer(download_dir: &Path, package: &DownloadedPackage) -> Result<(), CoreError> {
    let staged = download_dir.join(&package.file_name);
    if !staged.is_file() {
        return Err(lightproxy_update::install_failed(
            "安装包不存在，请重新下载",
        ));
    }
    if !staged
        .file_name()
        .map(|name| name.to_string_lossy().as_ref() == package.file_name)
        .unwrap_or(false)
    {
        return Err(lightproxy_update::verify_failed("安装包文件名不匹配"));
    }
    // 下载时已校验过，但文件可能被替换或损坏：执行前再按哈希和 PE 头复核一次。
    let actual = sha256_file(&staged)?;
    if actual != package.sha256 {
        return Err(lightproxy_update::verify_failed("安装包 SHA256 校验失败"));
    }
    if !has_mz(&staged) {
        return Err(lightproxy_update::verify_failed("安装包内容不是可执行程序"));
    }
    let destination = std::env::temp_dir().join(&package.file_name);
    copy_verified(&staged, &destination, &package.sha256)?;
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // 安装程序需要活过 LightProxy 退出；不等待它，也不共享控制台。
        let _child = std::process::Command::new(&destination)
            .creation_flags(DETACHED)
            .spawn()
            .map_err(|_| {
                let _ = fs::remove_file(&destination);
                lightproxy_update::install_failed("启动安装程序失败")
            })?;
    }
    #[cfg(not(windows))]
    {
        let _ = fs::remove_file(&destination);
        return Err(lightproxy_update::install_failed("仅支持 Windows"));
    }
    Ok(())
}

/// 复制到 `%TEMP%` 后，对「真正要执行的那份副本」再校验一次。
///
/// 校验过源文件不等于执行的是同一份字节：复制会按路径重新打开源文件，
/// 半拷贝或复制窗口内被替换都会让两者分叉。校验失败即删除副本，绝不交给系统执行。
fn copy_verified(source: &Path, destination: &Path, sha256: &str) -> Result<(), CoreError> {
    fs::copy(source, destination)
        .map_err(|_| lightproxy_update::install_failed("复制安装包失败"))?;
    let verified = sha256_file(destination)
        .map(|actual| actual == sha256 && has_mz(destination))
        .unwrap_or(false);
    if !verified {
        let _ = fs::remove_file(destination);
        return Err(lightproxy_update::verify_failed("安装包复制后校验失败"));
    }
    Ok(())
}

#[tauri::command]
pub fn open_release_page(version: String) -> Result<(), CoreError> {
    if !is_version(&version) {
        return Err(lightproxy_update::invalid_version(&version));
    }
    let url = release_url(&version);
    #[cfg(windows)]
    {
        let _child = std::process::Command::new("cmd")
            .args(["/C", "start", "", &url])
            .spawn()
            .map_err(|_| lightproxy_update::open_failed("打开发布页失败"))?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = url;
        Err(lightproxy_update::open_failed("仅支持 Windows"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lightproxy_core::{Desired, Protocol, Settings};

    fn state(proxy: ProxyState, protocol: Protocol) -> AppState {
        AppState {
            state: proxy,
            desired: Desired::On,
            active_profile_id: Some("p1".into()),
            last_health: None,
            message: None,
            profiles: vec![Profile {
                id: "p1".into(),
                name: "主用".into(),
                protocol,
                host: "127.0.0.1".into(),
                port: 7890,
            }],
            default_profile_id: None,
            settings: Settings::default(),
            snapshot: None,
        }
    }

    #[test]
    fn progress_percent_skips_unknown_totals_and_clamps_to_one_hundred() {
        assert_eq!(percent_of(0, None), None);
        assert_eq!(percent_of(0, Some(0)), None);
        assert_eq!(percent_of(0, Some(200)), Some(0));
        assert_eq!(percent_of(50, Some(200)), Some(25));
        assert_eq!(percent_of(200, Some(200)), Some(100));
        // 服务端声明的长度可能小于实际字节数：进度不能超过 100%。
        assert_eq!(percent_of(300, Some(200)), Some(100));
    }

    #[test]
    fn active_proxy_uses_socks5h_for_socks_profiles() {
        assert_eq!(
            active_proxy(&state(ProxyState::Off, Protocol::Socks5)),
            None
        );
        assert_eq!(
            active_proxy(&state(ProxyState::Enabling, Protocol::Socks5)),
            None
        );

        let http =
            active_proxy(&state(ProxyState::Degraded, Protocol::Http)).expect("降级时仍可复用代理");
        assert_eq!(http.url, "http://127.0.0.1:7890");
        assert_eq!(http.display, "127.0.0.1:7890");

        let socks = active_proxy(&state(ProxyState::On, Protocol::Socks5)).expect("SOCKS5 可复用");
        assert_eq!(socks.url, "socks5h://127.0.0.1:7890");
        assert_eq!(socks.display, "127.0.0.1:7890");
    }
}
