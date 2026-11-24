//! `export` and `setup`: the list of installed apps, saved on one computer
//! and installed on another.

use std::path::Path;

use suisha_core::managers::{self, Action};
use suisha_core::model::PackageRef;
use suisha_core::setup_file;

use crate::args::Args;
use crate::packages::chosen;
use crate::run::{self, print_json, say};
use crate::text::{self, count, tr};

pub fn export(args: &Args) -> Result<i32, String> {
    let mut all = Vec::new();
    for m in chosen(args)? {
        all.extend(managers::installed(m)?);
    }
    let file = setup_file::build(&all);
    let Some(path) = args.positional.first().filter(|p| p.as_str() != "-") else {
        // Without a file name the list goes to standard output.
        print_json(&file);
        return Ok(0);
    };
    let path = Path::new(path);
    if path.exists() {
        let name = path.display();
        let question = tr!(
            "{name} exists. Replace it?",
            "{name} は既にあります。置き換えますか？"
        );
        if !run::confirm(args, &question)? {
            say(args, tr!("Nothing was changed.", "何も変更していません。"));
            return Ok(1);
        }
    }
    setup_file::write(path, &file)?;
    let n = file.packages.len();
    let name = path.display();
    let apps = count(n, "app", "apps");
    say(
        args,
        tr!(
            "Saved {apps} to {name}.",
            "{n} 件を {name} に保存しました。"
        ),
    );
    Ok(0)
}

pub fn setup(args: &Args) -> Result<i32, String> {
    let Some(path) = args.positional.first() else {
        return Err(tr!(
            "Say which list to install from: suisha setup FILE",
            "一覧のファイルを指定してください：suisha setup ファイル"
        ));
    };
    let file = setup_file::read(Path::new(path)).map_err(|key| text::message(&key))?;
    let preview = setup_file::preview(&file);
    let with = |status: &str| -> Vec<&setup_file::PreviewEntry> {
        preview
            .entries
            .iter()
            .filter(|e| e.status == status)
            .collect()
    };
    let (missing, installed) = (with("missing"), with("installed"));
    let (unavailable, invalid) = (with("unavailable"), with("invalid"));

    let saved = text::local_time(&preview.exported_at);
    say(
        args,
        tr!("List saved on {saved}", "{saved} に保存された一覧"),
    );
    let (n, have) = (missing.len(), installed.len());
    say(
        args,
        tr!(
            "  {n} to install, {have} already installed",
            "  これから入れるもの {n} 件、既に入っているもの {have} 件"
        ),
    );
    if !unavailable.is_empty() {
        let n = unavailable.len();
        let mut names: Vec<&str> = unavailable
            .iter()
            .map(|e| e.package.manager.as_str())
            .collect();
        names.sort();
        names.dedup();
        let names = names.join(", ");
        say(
            args,
            tr!(
                "  {n} skipped: {names} is not installed on this computer",
                "  飛ばすもの {n} 件：このコンピューターに {names} が入っていません"
            ),
        );
    }
    if !invalid.is_empty() {
        let n = invalid.len();
        say(
            args,
            tr!(
                "  {n} skipped: not a package name",
                "  飛ばすもの {n} 件：パッケージの名前として読めません"
            ),
        );
    }
    for e in &missing {
        say(
            args,
            format!("    {}:{}", e.package.manager.as_str(), e.package.id),
        );
    }

    let refs: Vec<PackageRef> = missing.iter().map(|e| e.package.clone()).collect();
    let steps = managers::plan(Action::Install, &refs)?;
    run::execute(args, &steps, "install")
}
