//! Homebrew formulae and casks.

use std::collections::HashMap;

use serde_json::Value;

use super::text::lines;
use crate::model::{ManagerId, Package};

/// `brew list --formula --versions` / `brew list --cask --versions`:
/// `name 1.2.3 1.2.4`
pub fn parse_versions(out: &str) -> HashMap<String, String> {
    lines(out)
        .filter_map(|l| {
            let mut words = l.split_whitespace();
            let name = words.next()?.to_string();
            let version = words.last().unwrap_or("").to_string();
            Some((name, version))
        })
        .collect()
}

/// Formulae installed on request (`brew leaves --installed-on-request`),
/// with versions from `brew list --formula --versions`.
pub fn formulae_from(leaves: &str, versions: &str) -> Vec<Package> {
    let versions = parse_versions(versions);
    let mut list: Vec<Package> = lines(leaves)
        .map(|l| l.trim())
        .map(|full| {
            // Tapped formulae are printed as "user/tap/name".
            let short = full.rsplit('/').next().unwrap_or(full);
            let mut p = Package::new(ManagerId::Brew, full);
            p.name = short.to_string();
            p.version = versions.get(short).cloned().unwrap_or_default();
            p
        })
        .collect();
    list.sort_by(|a, b| a.name.cmp(&b.name));
    list
}

pub fn casks_from(versions: &str) -> Vec<Package> {
    let mut list: Vec<Package> = parse_versions(versions)
        .into_iter()
        .map(|(name, version)| {
            let mut p = Package::new(ManagerId::BrewCask, name);
            p.version = version;
            p
        })
        .collect();
    list.sort_by(|a, b| a.id.cmp(&b.id));
    list
}

fn version_text(v: Option<&Value>) -> String {
    match v {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(a)) => a
            .iter()
            .filter_map(|x| x.as_str())
            .next_back()
            .unwrap_or("")
            .to_string(),
        _ => String::new(),
    }
}

/// `brew outdated --json=v2`
pub fn parse_outdated(out: &str) -> Vec<Package> {
    let Ok(json) = serde_json::from_str::<Value>(out) else {
        return Vec::new();
    };
    let mut list = Vec::new();
    for (key, manager) in [
        ("formulae", ManagerId::Brew),
        ("casks", ManagerId::BrewCask),
    ] {
        let Some(items) = json.get(key).and_then(|v| v.as_array()) else {
            continue;
        };
        for item in items {
            let Some(name) = item.get("name").and_then(|v| v.as_str()) else {
                continue;
            };
            let mut p = Package::new(manager, name);
            p.version = version_text(item.get("installed_versions"));
            p.new_version = version_text(item.get("current_version"));
            list.push(p);
        }
    }
    list
}

/// `brew search --formula <q>` / `brew search --cask <q>`: one name per line.
/// Installed entries carry a check mark.
pub fn parse_search(out: &str, manager: ManagerId) -> Vec<Package> {
    lines(out)
        .map(|l| l.trim().trim_end_matches('✔').trim())
        .filter(|l| !l.starts_with("==>") && !l.contains(' '))
        .filter(|l| !l.is_empty())
        .map(|name| Package::new(manager, name))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formulae_and_casks() {
        let leaves = "gh\nnode@20\nhomebrew/core/wget\n";
        let versions = "gh 2.62.0\nnode@20 20.18.1\nwget 1.24.5\nopenssl@3 3.4.0\n";
        let list = formulae_from(leaves, versions);
        let names: Vec<_> = list.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, vec!["gh", "node@20", "wget"]);
        assert_eq!(list[2].id, "homebrew/core/wget");
        assert_eq!(list[2].version, "1.24.5");

        let casks = casks_from("firefox 133.0\nvisual-studio-code 1.95.3\n");
        assert_eq!(casks[1].id, "visual-studio-code");
        assert_eq!(casks[1].manager, Some(ManagerId::BrewCask));
    }

    #[test]
    fn outdated() {
        let out = r#"{"formulae":[{"name":"gh","installed_versions":["2.61.0"],"current_version":"2.62.0","pinned":false}],
                      "casks":[{"name":"firefox","installed_versions":["132.0"],"current_version":"133.0"},
                               {"name":"old","installed_versions":"1.0","current_version":"2.0"}]}"#;
        let list = parse_outdated(out);
        assert_eq!(list.len(), 3);
        assert_eq!(list[0].manager, Some(ManagerId::Brew));
        assert_eq!(list[0].version, "2.61.0");
        assert_eq!(list[1].new_version, "133.0");
        assert_eq!(list[2].version, "1.0");
        assert!(parse_outdated("not json").is_empty());
    }

    #[test]
    fn search() {
        let out = "==> Formulae\nwget ✔\nwget2\n\n";
        let list = parse_search(out, ManagerId::Brew);
        let names: Vec<_> = list.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(names, vec!["wget", "wget2"]);
        assert!(parse_search("No formulae or casks found for \"zz\".", ManagerId::Brew).is_empty());
    }
}
