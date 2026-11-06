mod common;

use lightproxy_update::{has_mz, parse_sha256, sha256_bytes, Source, UpdateClient};
use std::fs;

const VERSION: &str = "0.1.4";

fn asset(version: &str) -> String {
    format!("/download/v{version}/LightProxy_{version}_x64-setup.exe")
}

fn sha_asset(version: &str) -> String {
    format!("{}.sha256", asset(version))
}

#[test]
fn parses_sha256_files() {
    let hash = "a".repeat(64);
    assert_eq!(
        parse_sha256(&format!("{hash}  file.exe\n")),
        Some(hash.clone())
    );
    assert_eq!(parse_sha256(&hash.to_uppercase()), Some(hash));
    assert!(parse_sha256("not-a-hash").is_none());
    assert!(parse_sha256(&"z".repeat(64)).is_none());
}

#[test]
fn downloads_and_verifies_package_with_progress() {
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
    let dir = tempfile::tempdir().unwrap();
    let client = UpdateClient::new("0.1.2", None).unwrap();
    let sources = vec![Source::local("源1", &common::base(&server))];
    let mut progress = Vec::new();

    let package = client
        .download(VERSION, &sources, dir.path(), &mut |event| {
            progress.push(event)
        })
        .unwrap();

    assert_eq!(package.sha256, hash);
    assert_eq!(package.source, "源1");
    assert_eq!(package.size, body.len() as u64);
    let saved = dir.path().join(&package.file_name);
    assert_eq!(fs::metadata(&saved).unwrap().len(), body.len() as u64);
    assert!(has_mz(&saved));
    assert!(progress
        .windows(2)
        .all(|pair| pair[0].received <= pair[1].received));
    assert!(progress
        .iter()
        .any(|event| event.total == Some(body.len() as u64)));
    assert!(!dir
        .path()
        .join(format!("{}.part", package.file_name))
        .exists());
}

#[test]
fn falls_back_to_the_next_source() {
    let body = common::package_bytes();
    let hash = sha256_bytes(&body);
    let first = common::start(vec![
        (sha_asset(VERSION), 200, hash.clone().into_bytes()),
        (asset(VERSION), 500, b"boom".to_vec()),
    ]);
    let second = common::start(vec![
        (asset(VERSION), 200, body.clone()),
        (sha_asset(VERSION), 200, hash.clone().into_bytes()),
    ]);
    let dir = tempfile::tempdir().unwrap();
    let client = UpdateClient::new("0.1.2", None).unwrap();
    let sources = vec![
        Source::local("源1", &common::base(&first)),
        Source::local("源2", &common::base(&second)),
    ];

    let package = client
        .download(VERSION, &sources, dir.path(), &mut |_| {})
        .unwrap();

    assert_eq!(package.source, "源2");
    assert!(first
        .hits
        .lock()
        .unwrap()
        .iter()
        .any(|path| path == &asset(VERSION)));
    assert!(second
        .hits
        .lock()
        .unwrap()
        .iter()
        .any(|path| path == &asset(VERSION)));
}

#[test]
fn rejects_missing_checksum() {
    let body = common::package_bytes();
    let server = common::start(vec![(asset(VERSION), 200, body)]);
    let dir = tempfile::tempdir().unwrap();
    let client = UpdateClient::new("0.1.2", None).unwrap();
    let sources = vec![Source::local("源1", &common::base(&server))];

    let error = client
        .download(VERSION, &sources, dir.path(), &mut |_| {})
        .unwrap_err();

    assert_eq!(error.code, "update_download_failed");
}

#[test]
fn rejects_hash_mismatch_and_removes_partial_file() {
    let body = common::package_bytes();
    let wrong = sha256_bytes(b"other content");
    let server = common::start(vec![
        (asset(VERSION), 200, body),
        (sha_asset(VERSION), 200, wrong.into_bytes()),
    ]);
    let dir = tempfile::tempdir().unwrap();
    let client = UpdateClient::new("0.1.2", None).unwrap();
    let sources = vec![Source::local("源1", &common::base(&server))];

    let error = client
        .download(VERSION, &sources, dir.path(), &mut |_| {})
        .unwrap_err();

    assert_eq!(error.code, "update_verify_failed");
    let file = format!("LightProxy_{VERSION}_x64-setup.exe");
    assert!(!dir.path().join(&file).exists());
    assert!(!dir.path().join(format!("{file}.part")).exists());
}

#[test]
fn rejects_payload_without_mz_header() {
    let body = b"not-an-executable".to_vec();
    let hash = sha256_bytes(&body);
    let server = common::start(vec![
        (asset(VERSION), 200, body),
        (sha_asset(VERSION), 200, hash.into_bytes()),
    ]);
    let dir = tempfile::tempdir().unwrap();
    let client = UpdateClient::new("0.1.2", None).unwrap();
    let sources = vec![Source::local("源1", &common::base(&server))];

    let error = client
        .download(VERSION, &sources, dir.path(), &mut |_| {})
        .unwrap_err();

    assert_eq!(error.code, "update_verify_failed");
}

#[test]
fn reuses_a_verified_package_without_downloading() {
    let body = common::package_bytes();
    let hash = sha256_bytes(&body);
    let server = common::start(vec![(sha_asset(VERSION), 200, hash.clone().into_bytes())]);
    let dir = tempfile::tempdir().unwrap();
    let file_name = format!("LightProxy_{VERSION}_x64-setup.exe");
    fs::write(dir.path().join(&file_name), &body).unwrap();
    let client = UpdateClient::new("0.1.2", None).unwrap();
    let sources = vec![Source::local("源1", &common::base(&server))];

    let package = client
        .download(VERSION, &sources, dir.path(), &mut |_| {})
        .unwrap();

    assert!(package.source.contains("缓存"));
    assert!(!server
        .hits
        .lock()
        .unwrap()
        .iter()
        .any(|path| path == &asset(VERSION)));
}

#[test]
fn rejects_an_invalid_proxy_with_an_update_error() {
    let error = UpdateClient::new("0.1.2", Some("not a url"))
        .err()
        .expect("无效代理地址应当构造失败");

    assert_eq!(error.stage, "update");
    assert_eq!(
        error.code, "update_check_failed",
        "构造失败必须落在检查错误的 code 上"
    );
    assert!(error.hint.is_some(), "构造失败应带修复提示");
    assert!(
        error.message.contains("代理地址"),
        "实际为 {}",
        error.message
    );
}

#[test]
fn check_reports_the_latest_release_and_source() {
    let feed = r#"<feed>
        <entry><link rel="alternate" href="https://github.com/BeafJerky/LightProxy/releases/tag/v0.1.2"/></entry>
        <entry><link rel="alternate" href="https://github.com/BeafJerky/LightProxy/releases/tag/v0.1.4"/></entry>
    </feed>"#;
    let server = common::start(vec![(
        "/releases.atom".into(),
        200,
        feed.as_bytes().to_vec(),
    )]);
    let client = UpdateClient::new("0.1.2", None).unwrap();
    let sources = vec![Source::local("源1", &common::base(&server))];

    let outcome = client.check(&sources).unwrap();

    assert!(outcome.available);
    assert_eq!(outcome.latest_version, "0.1.4");
    assert_eq!(outcome.asset_name, "LightProxy_0.1.4_x64-setup.exe");
    assert_eq!(outcome.source, "源1");
}

#[test]
fn check_falls_back_and_fails_when_no_source_has_versions() {
    let first = common::start(vec![("/releases.atom".into(), 500, b"boom".to_vec())]);
    let feed = r#"<feed>
        <entry><link rel="alternate" href="https://github.com/BeafJerky/LightProxy/releases/tag/v0.1.4"/></entry>
    </feed>"#;
    let second = common::start(vec![(
        "/releases.atom".into(),
        200,
        feed.as_bytes().to_vec(),
    )]);
    let client = UpdateClient::new("0.1.2", None).unwrap();
    let sources = vec![
        Source::local("源1", &common::base(&first)),
        Source::local("源2", &common::base(&second)),
    ];
    assert_eq!(client.check(&sources).unwrap().source, "源2");

    let empty = common::start(vec![("/releases.atom".into(), 200, b"<feed/>".to_vec())]);
    let sources = vec![Source::local("源3", &common::base(&empty))];
    let error = client.check(&sources).unwrap_err();
    assert_eq!(error.code, "update_check_failed");
}

const FEED: &str = r#"<feed>
    <entry><link rel="alternate" href="https://github.com/BeafJerky/LightProxy/releases/tag/v0.1.4"/></entry>
</feed>"#;

#[test]
fn refuses_a_redirect_to_plain_http() {
    // https-only 的硬约束要在运行时生效：镜像把 Atom 跳转到明文地址时必须失败。
    let server = common::start_replies(vec![
        (
            "/releases.atom".into(),
            common::Reply::new(302, Vec::new()).header("Location", "{base}/plain.atom"),
        ),
        (
            "/plain.atom".into(),
            common::Reply::new(200, FEED.as_bytes().to_vec()),
        ),
    ]);
    let client = UpdateClient::new("0.1.2", None).unwrap();
    let sources = vec![Source::local("源1", &common::base(&server))];

    let error = client.check(&sources).unwrap_err();

    assert_eq!(error.code, "update_check_failed");
    assert!(error.message.contains("跳转"), "实际为 {}", error.message);
    assert!(
        !server
            .hits
            .lock()
            .unwrap()
            .iter()
            .any(|path| path == "/plain.atom"),
        "明文跳转目标不应被请求"
    );
}

#[test]
fn rejects_an_oversized_atom() {
    let body = vec![b'x'; 600 * 1024];
    let server = common::start(vec![("/releases.atom".into(), 200, body)]);
    let client = UpdateClient::new("0.1.2", None).unwrap();
    let sources = vec![Source::local("源1", &common::base(&server))];

    let error = client.check(&sources).unwrap_err();

    assert_eq!(error.code, "update_check_failed");
    assert!(error.message.contains("过大"), "实际为 {}", error.message);
}

#[test]
fn rejects_an_oversized_atom_without_a_declared_length() {
    // 服务端不声明 Content-Length 时，仍然要在读取过程中按上限截断。
    let server = common::start_replies(vec![(
        "/releases.atom".into(),
        common::Reply::new(200, vec![b'x'; 600 * 1024]).without_content_length(),
    )]);
    let client = UpdateClient::new("0.1.2", None).unwrap();
    let sources = vec![Source::local("源1", &common::base(&server))];

    let error = client.check(&sources).unwrap_err();

    assert_eq!(error.code, "update_check_failed");
    assert!(error.message.contains("过大"), "实际为 {}", error.message);
}

#[test]
fn rejects_an_oversized_checksum_before_requesting_the_package() {
    let server = common::start(vec![(sha_asset(VERSION), 200, vec![b'0'; 600 * 1024])]);
    let dir = tempfile::tempdir().unwrap();
    let client = UpdateClient::new("0.1.2", None).unwrap();
    let sources = vec![Source::local("源1", &common::base(&server))];

    let error = client
        .download(VERSION, &sources, dir.path(), &mut |_| {})
        .unwrap_err();

    assert_eq!(error.code, "update_download_failed");
    assert!(
        !server
            .hits
            .lock()
            .unwrap()
            .iter()
            .any(|path| path == &asset(VERSION)),
        "校验和超限后不应再请求安装包"
    );
    assert!(dir.path().read_dir().unwrap().next().is_none());
}

#[test]
fn rejects_a_package_with_an_oversized_declared_length() {
    let body = common::package_bytes();
    let hash = sha256_bytes(&body);
    let server = common::start_replies(vec![
        (
            sha_asset(VERSION),
            common::Reply::new(200, hash.into_bytes()),
        ),
        (
            asset(VERSION),
            common::Reply::new(200, body)
                .header("Content-Length", (600u64 * 1024 * 1024).to_string()),
        ),
    ]);
    let dir = tempfile::tempdir().unwrap();
    let client = UpdateClient::new("0.1.2", None).unwrap();
    let sources = vec![Source::local("源1", &common::base(&server))];

    let error = client
        .download(VERSION, &sources, dir.path(), &mut |_| {})
        .unwrap_err();

    assert_eq!(error.code, "update_download_failed");
    assert!(error.message.contains("上限"), "实际为 {}", error.message);
    let file = format!("LightProxy_{VERSION}_x64-setup.exe");
    assert!(!dir.path().join(&file).exists());
    assert!(!dir.path().join(format!("{file}.part")).exists());
}
