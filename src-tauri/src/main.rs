// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // With a command (`suisha update`, `suisha --auto-update` and so on) this
    // works like the command line program and opens no window.
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().is_some_and(|a| suisha_cli::is_command(a)) {
        std::process::exit(suisha_cli::run(&args));
    }
    suisha_lib::run()
}
