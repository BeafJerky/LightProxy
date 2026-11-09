use lightproxy_core::backend::{MemoryBackend, ProxyBackend};
use lightproxy_core::model::EnvVar;

fn var(name: &str, value: Option<&str>) -> EnvVar {
    EnvVar {
        name: name.into(),
        value: value.map(|v| v.into()),
        expand: false,
    }
}

#[test]
fn read_env_returns_all_four_canonical_names() {
    let backend = MemoryBackend::new(Default::default());
    backend.set_env(vec![var("HTTP_PROXY", Some("http://x:1"))]);
    let vars = backend.read_env().unwrap();
    assert_eq!(vars.len(), 4);
    let http = vars
        .iter()
        .find(|v| v.name.eq_ignore_ascii_case("http_proxy"))
        .unwrap();
    assert_eq!(http.value.as_deref(), Some("http://x:1"));
    assert!(vars
        .iter()
        .any(|v| v.name == "https_proxy" && v.value.is_none()));
}

#[test]
fn write_env_sets_and_deletes_by_canonical_name() {
    let backend = MemoryBackend::new(Default::default());
    backend.set_env(vec![var("http_proxy", Some("http://old:1"))]);
    backend
        .write_env(&[
            var("http_proxy", Some("http://new:2")),
            var("https_proxy", None),
        ])
        .unwrap();
    let vars = backend.env_vars();
    assert_eq!(
        vars.iter()
            .find(|v| v.name == "http_proxy")
            .unwrap()
            .value
            .as_deref(),
        Some("http://new:2")
    );
    assert!(!vars
        .iter()
        .any(|v| v.name.eq_ignore_ascii_case("https_proxy")));
    assert_eq!(backend.env_writes(), 1);
}

#[test]
fn notify_env_counts() {
    let backend = MemoryBackend::new(Default::default());
    backend.notify_env().unwrap();
    assert_eq!(backend.env_notifies(), 1);
}

#[test]
fn fail_next_env_write_errors_once() {
    let backend = MemoryBackend::new(Default::default());
    backend.fail_next_env_write();
    let err = backend
        .write_env(&[var("http_proxy", Some("x"))])
        .unwrap_err();
    assert_eq!(err.code, "env_write_failed");
    backend.write_env(&[var("http_proxy", Some("x"))]).unwrap();
}

#[test]
fn fail_env_reads_errors() {
    let backend = MemoryBackend::new(Default::default());
    backend.fail_env_reads(true);
    assert_eq!(backend.read_env().unwrap_err().code, "env_read_failed");
}
