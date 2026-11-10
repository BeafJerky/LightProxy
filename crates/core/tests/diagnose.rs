use lightproxy_core::diagnose::{recommend, summarize};
use lightproxy_core::{DiagnoseReport, PathResult, PathStage, Recommendation};

fn result(ok: bool, stage: PathStage) -> PathResult {
    PathResult {
        ok,
        stage,
        latency_ms: 1,
        error: if ok { None } else { Some("失败".into()) },
    }
}

#[test]
fn recommendation_prefers_http_then_system_socks_then_remote_socks5() {
    let pass = PathStage::Http;
    let rows = [
        (true, true, true, Recommendation::Http),
        (true, true, false, Recommendation::Http),
        (true, false, true, Recommendation::Http),
        (true, false, false, Recommendation::Http),
        (false, true, true, Recommendation::Socks5System),
        (false, false, true, Recommendation::Socks5System),
        (false, true, false, Recommendation::Socks5EnvOnly),
        (false, false, false, Recommendation::Unavailable),
    ];
    for (http, remote, system, expected) in rows {
        let http_result = result(http, pass);
        let remote_result = result(remote, pass);
        let system_result = result(system, pass);
        assert_eq!(
            recommend(&http_result, &remote_result, &system_result),
            expected,
            "http={http}, remote={remote}, system={system}"
        );
    }
}

#[test]
fn summary_explains_each_recommendation_and_two_failure_causes() {
    let ok = result(true, PathStage::Http);
    let tunnel = result(false, PathStage::Tunnel);
    let proxy = result(false, PathStage::Proxy);

    assert_eq!(
        summarize(Recommendation::Http, &ok, &ok, &ok),
        "HTTP 通道可用，建议使用 HTTP。"
    );
    assert_eq!(
        summarize(Recommendation::Socks5System, &tunnel, &ok, &ok),
        "HTTP 通道不可用，系统 SOCKS 可用，建议使用 SOCKS5。"
    );
    assert_eq!(
        summarize(Recommendation::Socks5EnvOnly, &tunnel, &ok, &tunnel),
        "只有 SOCKS5 远程解析可用；系统代理会按本机解析，建议仅启用环境变量同步。"
    );
    assert_eq!(
        summarize(Recommendation::Unavailable, &proxy, &proxy, &proxy),
        "代理端口不可达，请检查地址与端口。"
    );
    assert_eq!(
        summarize(Recommendation::Unavailable, &tunnel, &proxy, &proxy),
        "代理端口可达，但转发或解析失败，请检查代理上游。"
    );
}

#[test]
fn diagnose_report_serializes_with_camel_case_fields() {
    let report = DiagnoseReport {
        http: result(true, PathStage::Http),
        socks5_domain: result(false, PathStage::Tunnel),
        system_socks: result(false, PathStage::Tls),
        recommendation: Recommendation::Socks5EnvOnly,
        summary: "只有 SOCKS5 远程解析可用。".into(),
    };

    let json = serde_json::to_value(&report).unwrap();

    assert_eq!(json["http"]["latencyMs"], 1);
    assert_eq!(json["socks5Domain"]["stage"], "tunnel");
    assert_eq!(json["systemSocks"]["stage"], "tls");
    assert_eq!(json["recommendation"], "socks5EnvOnly");
}
