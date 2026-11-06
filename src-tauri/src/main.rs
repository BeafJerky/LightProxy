#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if std::env::args().any(|arg| arg == "--restore-snapshot") {
        std::process::exit(lightproxy_lib::restore_snapshot_cli());
    }
    lightproxy_lib::run();
}
