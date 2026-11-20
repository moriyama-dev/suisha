// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("--auto-update") => std::process::exit(suisha_lib::background::auto_update()),
        Some("--self-update") => std::process::exit(suisha_lib::background::self_update()),
        Some("--self-check") => std::process::exit(suisha_lib::background::self_check()),
        Some("--version") => println!("suisha {}", env!("CARGO_PKG_VERSION")),
        _ => suisha_lib::run(),
    }
}
