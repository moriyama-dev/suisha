//! The `suisha` command.

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    // `suisha` alone opens the window where one is installed and a desktop
    // is there to show it. That is what the command did before the window
    // moved to `suisha-gui`, and launchers made back then still rely on it.
    if args.is_empty()
        && let Some(window) = suisha_cli::window_program()
    {
        use std::os::unix::process::CommandExt;
        let error = std::process::Command::new(&window).exec();
        eprintln!("suisha: {}: {error}", window.display());
        std::process::exit(1);
    }
    std::process::exit(suisha_cli::run(&args));
}
