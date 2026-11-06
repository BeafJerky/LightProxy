use lightproxy_core::CoreError;

pub const STAGE: &str = "update";

pub fn check_failed(message: impl Into<String>) -> CoreError {
    CoreError::new("update_check_failed", STAGE, message).with_hint("检查网络或代理后重试")
}

pub fn not_available() -> CoreError {
    CoreError::new("update_not_available", STAGE, "请求的版本不是可用更新").with_hint("先检查更新")
}

pub fn busy() -> CoreError {
    CoreError::new("update_busy", STAGE, "已有更新任务正在进行").with_hint("等待当前操作完成")
}

pub fn download_failed(message: impl Into<String>) -> CoreError {
    CoreError::new("update_download_failed", STAGE, message).with_hint("重试或切换网络")
}

pub fn verify_failed(message: impl Into<String>) -> CoreError {
    CoreError::new("update_verify_failed", STAGE, message).with_hint("删除后重新下载")
}

pub fn no_package() -> CoreError {
    CoreError::new("update_no_package", STAGE, "尚未下载更新安装包").with_hint("先下载更新")
}

pub fn install_failed(message: impl Into<String>) -> CoreError {
    CoreError::new("update_install_failed", STAGE, message).with_hint("检查安全软件后重试")
}

pub fn open_failed(message: impl Into<String>) -> CoreError {
    CoreError::new("update_open_failed", STAGE, message).with_hint("重试或手动打开发布页")
}

pub fn invalid_version(version: &str) -> CoreError {
    CoreError::new(
        "update_not_available",
        STAGE,
        format!("版本号无效：{version}"),
    )
    .with_hint("先检查更新")
}
