//! `list`, `updates`, `search`, `install`, `remove`, `update` and `cleanup`.

use std::collections::HashSet;

use suisha_core::managers::{self, Action};
use suisha_core::model::{ManagerId, Package, PackageRef, valid_package_id};
use suisha_core::privileged;

use crate::args::{Args, manager_from};
use crate::run::{self, print_json, say, warn};
use crate::text::{count, table, tr};

/// The package managers a command works on: all that are installed, or the
/// ones named with `--manager`.
pub fn chosen(args: &Args) -> Result<Vec<ManagerId>, String> {
    let available = managers::available();
    if available.is_empty() {
        return Err(tr!(
            "No package manager Suisha knows was found on this computer.",
            "このコンピューターには、Suisha が扱うパッケージマネージャーが見つかりません。"
        ));
    }
    if args.managers.is_empty() {
        return Ok(available);
    }
    for m in &args.managers {
        if !available.contains(m) {
            let name = m.as_str();
            return Err(tr!(
                "{name} is not installed on this computer.",
                "{name} はこのコンピューターに入っていません。"
            ));
        }
    }
    Ok(args.managers.clone())
}

/// How a package is written on the command line: `apt:git`.
pub fn label(p: &Package) -> String {
    match p.manager {
        Some(m) => format!("{}:{}", m.as_str(), p.id),
        None => p.id.clone(),
    }
}

/// Read one list from every chosen manager. A manager that fails is
/// reported and the others are still shown; the flag tells that one failed.
fn collect(
    managers: &[ManagerId],
    read: impl Fn(ManagerId) -> Result<Vec<Package>, String>,
) -> (Vec<Package>, bool) {
    let mut all = Vec::new();
    let mut failed = false;
    for m in managers {
        match read(*m) {
            Ok(list) => all.extend(list),
            Err(e) => {
                warn(format!("{}: {e}", m.as_str()));
                failed = true;
            }
        }
    }
    (all, failed)
}

pub fn list(args: &Args) -> Result<i32, String> {
    let (all, failed) = collect(&chosen(args)?, managers::installed);
    if args.json {
        print_json(&all);
    } else if all.is_empty() {
        println!(
            "{}",
            tr!("Nothing is installed.", "入っているものはありません。")
        );
    } else {
        let rows: Vec<Vec<String>> = all
            .iter()
            .map(|p| {
                let mut row = vec![label(p), p.version.clone()];
                if p.name != p.id {
                    row.push(p.name.clone());
                }
                row
            })
            .collect();
        print!("{}", table(&rows));
    }
    Ok(i32::from(failed))
}

fn held_note(reason: &str) -> String {
    match reason {
        "phased" => tr!(
            "waiting for Ubuntu's phased rollout",
            "Ubuntu の段階的な配信を待っています"
        ),
        "kept-back" => tr!("kept back by apt", "apt が保留しています"),
        _ => String::new(),
    }
}

pub fn updates(args: &Args) -> Result<i32, String> {
    let (all, failed) = collect(&chosen(args)?, managers::updates);
    if args.json {
        print_json(&all);
        return Ok(i32::from(failed));
    }
    let rows: Vec<Vec<String>> = all
        .iter()
        .map(|p| {
            let versions = match (p.version.is_empty(), p.new_version.is_empty()) {
                (false, false) => format!("{} -> {}", p.version, p.new_version),
                (true, false) => p.new_version.clone(),
                _ => p.version.clone(),
            };
            vec![label(p), versions, held_note(&p.held)]
        })
        .collect();
    print!("{}", table(&rows));
    let ready = all.iter().filter(|p| p.held.is_empty()).count();
    let waiting = all.len() - ready;
    if ready == 0 {
        println!("{}", tr!("Everything is up to date.", "すべて最新です。"));
    } else {
        let n = count(ready, "update", "updates");
        println!(
            "{}",
            tr!(
                "{n} can be installed: suisha update",
                "{ready} 件の更新を入れられます：suisha update"
            )
        );
    }
    if waiting > 0 {
        println!(
            "{}",
            tr!(
                "{waiting} more will be installed by apt when their turn comes; nothing to do for those.",
                "ほかに {waiting} 件は、順番が来たら apt が入れます。何もしなくてかまいません。"
            )
        );
    }
    Ok(i32::from(failed))
}

fn shorten(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let cut: String = text.chars().take(max.saturating_sub(1)).collect();
    format!("{}…", cut.trim_end())
}

pub fn search(args: &Args) -> Result<i32, String> {
    let query = args.positional.join(" ");
    if query.trim().is_empty() {
        return Err(tr!(
            "Say what to search for: suisha search NAME",
            "探す名前を指定してください：suisha search 名前"
        ));
    }
    let limit = args.limit.unwrap_or(10);
    let (all, failed) = collect(&chosen(args)?, |m| {
        managers::search(m, &query).map(|mut list| {
            list.truncate(limit);
            list
        })
    });
    if args.json {
        print_json(&all);
        return Ok(i32::from(failed));
    }
    if all.is_empty() {
        println!("{}", tr!("Nothing was found.", "見つかりませんでした。"));
        return Ok(i32::from(failed));
    }
    let rows: Vec<Vec<String>> = all
        .iter()
        .map(|p| {
            let about = if p.description.is_empty() && p.name != p.id {
                &p.name
            } else {
                &p.description
            };
            vec![label(p), p.version.clone(), shorten(about, 70)]
        })
        .collect();
    print!("{}", table(&rows));
    let example = label(&all[0]);
    println!(
        "{}",
        tr!(
            "Install with: suisha install {example}",
            "インストール：suisha install {example}"
        )
    );
    Ok(i32::from(failed))
}

/// A package named on the command line: `apt:git`, or just `git` where the
/// package manager can be found out.
#[derive(Debug, PartialEq)]
pub struct Wanted {
    pub manager: Option<ManagerId>,
    pub id: String,
}

pub fn parse_wanted(text: &str) -> Result<Wanted, String> {
    let (manager, id) = match text.split_once(':') {
        // `libfoo:amd64` is an apt package, not a package manager called libfoo.
        Some((prefix, rest)) if manager_from(prefix).is_some() => (manager_from(prefix), rest),
        _ => (None, text),
    };
    if !valid_package_id(id) {
        return Err(tr!(
            "{text} is not a package name.",
            "{text} はパッケージの名前ではありません。"
        ));
    }
    Ok(Wanted {
        manager,
        id: id.to_string(),
    })
}

fn parse_all(args: &Args) -> Result<Vec<Wanted>, String> {
    args.positional.iter().map(|t| parse_wanted(t)).collect()
}

/// Tapped Homebrew formulae are listed by their short name.
fn same_id(p: &Package, id: &str) -> bool {
    p.id == id || p.id.rsplit('/').next() == Some(id) || id.rsplit('/').next() == Some(&p.id)
}

/// Find each wanted package in `pool`. A name without a package manager
/// must match exactly one entry.
pub fn resolve(
    wanted: &[Wanted],
    pool: &[Package],
    missing: impl Fn(&str) -> String,
) -> Result<Vec<PackageRef>, String> {
    let mut refs = Vec::new();
    for w in wanted {
        let matches: Vec<&Package> = pool
            .iter()
            .filter(|p| same_id(p, &w.id) && (w.manager.is_none() || p.manager == w.manager))
            .collect();
        match matches.as_slice() {
            [] => return Err(missing(&w.id)),
            [one] => refs.push(PackageRef::from(*one)),
            many => {
                let id = &w.id;
                let choices: Vec<String> = many.iter().map(|p| label(p)).collect();
                let choices = choices.join(", ");
                return Err(tr!(
                    "{id} could be any of these; name one: {choices}",
                    "{id} は次のどれにも当てはまります。どれか1つを指定してください：{choices}"
                ));
            }
        }
    }
    Ok(refs)
}

/// The managers to read for resolving: the ones named, or all the chosen
/// ones as soon as one package comes without a manager.
fn pool_managers(args: &Args, wanted: &[Wanted]) -> Result<Vec<ManagerId>, String> {
    let all = chosen(args)?;
    if wanted.iter().any(|w| w.manager.is_none()) {
        return Ok(all);
    }
    let named: HashSet<ManagerId> = wanted.iter().filter_map(|w| w.manager).collect();
    for m in &named {
        if managers::binary(*m).is_none() {
            let name = m.as_str();
            return Err(tr!(
                "{name} is not installed on this computer.",
                "{name} はこのコンピューターに入っていません。"
            ));
        }
    }
    Ok(ManagerId::ALL
        .into_iter()
        .filter(|m| named.contains(m))
        .collect())
}

pub fn install(args: &Args) -> Result<i32, String> {
    let wanted = parse_all(args)?;
    if wanted.is_empty() {
        return Err(tr!(
            "Say what to install, for example: suisha install apt:htop",
            "インストールするものを指定してください。例：suisha install apt:htop"
        ));
    }
    let mut refs = Vec::new();
    for w in &wanted {
        let id = &w.id;
        let Some(manager) = w.manager else {
            return Err(tr!(
                "Say where {id} should come from, for example apt:{id}. `suisha search {id}` shows the choices.",
                "{id} の入手先を指定してください（例：apt:{id}）。`suisha search {id}` で候補を見られます。"
            ));
        };
        if managers::binary(manager).is_none() {
            let name = manager.as_str();
            return Err(tr!(
                "{name} is not installed on this computer.",
                "{name} はこのコンピューターに入っていません。"
            ));
        }
        let mut package = PackageRef {
            manager,
            id: id.clone(),
            scope: String::new(),
            origin: String::new(),
            classic: false,
        };
        // A snap may need `--classic`, and a Flatpak app comes from a
        // remote; the search result knows both.
        if matches!(manager, ManagerId::Snap | ManagerId::Flatpak)
            && let Some(found) = managers::search(manager, id)
                .unwrap_or_default()
                .into_iter()
                .find(|p| p.id == *id)
        {
            package.origin = found.origin;
            package.classic = found.classic;
        }
        refs.push(package);
    }
    let steps = managers::plan(Action::Install, &refs)?;
    run::execute(args, &steps, "install")
}

pub fn remove(args: &Args) -> Result<i32, String> {
    let wanted = parse_all(args)?;
    if wanted.is_empty() {
        return Err(tr!(
            "Say what to remove, for example: suisha remove apt:htop",
            "削除するものを指定してください。例：suisha remove apt:htop"
        ));
    }
    let (pool, _) = collect(&pool_managers(args, &wanted)?, managers::installed);
    // Only what `suisha list` shows can be removed: dependencies and the
    // base system are not in that list, so they cannot go by accident.
    let refs = resolve(&wanted, &pool, |id| {
        tr!(
            "{id} is not in `suisha list`, so Suisha does not remove it.",
            "{id} は `suisha list` に無いので、Suisha では削除しません。"
        )
    })?;
    let steps = managers::plan(Action::Remove, &refs)?;
    run::execute(args, &steps, "remove")
}

fn pending(managers: &[ManagerId]) -> Vec<Package> {
    managers
        .iter()
        .flat_map(|m| managers::updates(*m).unwrap_or_default())
        .filter(|p| p.held.is_empty())
        .collect()
}

fn update_all(args: &Args) -> Result<i32, String> {
    let chosen = chosen(args)?;
    let before = pending(&chosen);
    for m in &chosen {
        let n = before.iter().filter(|p| p.manager == Some(*m)).count();
        if n > 0 {
            let name = m.as_str();
            let updates = count(n, "update", "updates");
            say(
                args,
                tr!("{name}: {updates} waiting", "{name}：{n} 件の更新"),
            );
        }
    }
    // The package lists may be older than what the servers have, so the
    // update runs even when nothing is known to be waiting.
    let steps = managers::plan_update_all(&chosen, true, true)?;
    if steps.is_empty() {
        say(args, tr!("Nothing to do.", "することはありません。"));
        return Ok(0);
    }
    if !run::prepare(args, &steps)? {
        return Ok(if args.dry_run { 0 } else { 1 });
    }
    let result = run::perform(args, &steps, "update-all");
    let after: HashSet<String> = pending(&chosen)
        .iter()
        .map(crate::packages::label)
        .collect();
    let installed = before.iter().filter(|p| !after.contains(&label(p))).count();
    let code = run::finish(args, &result);
    if installed > 0 {
        let updates = count(installed, "update was", "updates were");
        say(
            args,
            tr!("{updates} installed.", "{installed} 件の更新を入れました。"),
        );
    }
    Ok(code)
}

pub fn update(args: &Args) -> Result<i32, String> {
    let wanted = parse_all(args)?;
    if wanted.is_empty() {
        return update_all(args);
    }
    let (pool, _) = collect(&pool_managers(args, &wanted)?, managers::updates);
    let refs = resolve(&wanted, &pool, |id| {
        tr!(
            "{id} has no update waiting (see `suisha updates`).",
            "{id} には更新がありません（`suisha updates` で確認できます）。"
        )
    })?;
    let steps = managers::plan(Action::Update, &refs)?;
    run::execute(args, &steps, "update")
}

/// Remove packages nothing needs any more (`apt autoremove`).
pub fn cleanup(args: &Args) -> Result<i32, String> {
    let unneeded = suisha_core::system::status().unneeded;
    if unneeded.is_empty() {
        say(
            args,
            tr!(
                "No package is left over; nothing to clean up.",
                "不要になったパッケージはありません。"
            ),
        );
        return Ok(0);
    }
    let n = unneeded.len();
    let packages = count(n, "package is", "packages are");
    say(
        args,
        tr!(
            "{packages} no longer needed:",
            "不要になったパッケージが {n} 件あります："
        ),
    );
    say(args, format!("  {}", unneeded.join(" ")));
    let steps = vec![privileged::autoremove_step()?];
    run::execute(args, &steps, "clean-up")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wanted(text: &str) -> Wanted {
        parse_wanted(text).unwrap()
    }

    #[test]
    fn reads_package_names() {
        assert_eq!(
            wanted("apt:git"),
            Wanted {
                manager: Some(ManagerId::Apt),
                id: "git".into()
            }
        );
        assert_eq!(wanted("htop").manager, None);
        // An architecture suffix is part of the name.
        assert_eq!(wanted("libfoo:amd64").id, "libfoo:amd64");
        assert_eq!(wanted("apt:libfoo:amd64").id, "libfoo:amd64");
        assert_eq!(wanted("cask:firefox").manager, Some(ManagerId::BrewCask));
        // Nothing that could be read as an option reaches a package manager.
        assert!(parse_wanted("apt:--purge").is_err());
        assert!(parse_wanted("apt:").is_err());
        assert!(parse_wanted("a b").is_err());
    }

    fn pool() -> Vec<Package> {
        let mut flatpak = Package::new(ManagerId::Flatpak, "org.gimp.GIMP");
        flatpak.scope = "user".into();
        flatpak.origin = "flathub".into();
        vec![
            Package::new(ManagerId::Apt, "git"),
            Package::new(ManagerId::Apt, "ffmpeg"),
            Package::new(ManagerId::Snap, "ffmpeg"),
            Package::new(ManagerId::Brew, "someone/tap/tool"),
            flatpak,
        ]
    }

    #[test]
    fn finds_packages_by_name() {
        let missing = |id: &str| format!("no {id}");
        let refs = resolve(&[wanted("git")], &pool(), missing).unwrap();
        assert_eq!(refs[0].manager, ManagerId::Apt);

        // What the package manager needs later comes from the list.
        let refs = resolve(&[wanted("org.gimp.GIMP")], &pool(), missing).unwrap();
        assert_eq!(refs[0].scope, "user");
        assert_eq!(refs[0].origin, "flathub");

        let refs = resolve(&[wanted("brew:tool")], &pool(), missing).unwrap();
        assert_eq!(refs[0].id, "someone/tap/tool");
    }

    #[test]
    fn asks_when_a_name_fits_two_packages() {
        let missing = |id: &str| format!("no {id}");
        let error = resolve(&[wanted("ffmpeg")], &pool(), missing).unwrap_err();
        assert!(error.contains("apt:ffmpeg") && error.contains("snap:ffmpeg"));
        let refs = resolve(&[wanted("snap:ffmpeg")], &pool(), missing).unwrap();
        assert_eq!(refs[0].manager, ManagerId::Snap);
        assert_eq!(
            resolve(&[wanted("apt:bash")], &pool(), missing).unwrap_err(),
            "no bash"
        );
    }

    #[test]
    fn shortens_long_descriptions() {
        assert_eq!(shorten("short", 10), "short");
        assert_eq!(shorten("a long description", 7), "a long…");
    }
}
