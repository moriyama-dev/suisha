//! Homebrew on Linux (formulae).

use std::collections::HashMap;

use serde_json::Value;

use super::text::lines;
use crate::model::{ManagerId, Package};

/// `brew list --formula --versions`: `name 1.2.3 1.2.4`
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

/// `brew outdated --formula --json=v2`
pub fn parse_outdated(out: &str) -> Vec<Package> {
    let Ok(json) = serde_json::from_str::<Value>(out) else {
        return Vec::new();
    };
    let mut list = Vec::new();
    for item in json
        .get("formulae")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
    {
        let Some(name) = item.get("name").and_then(|v| v.as_str()) else {
            continue;
        };
        let mut p = Package::new(ManagerId::Brew, name);
        p.version = version_text(item.get("installed_versions"));
        p.new_version = version_text(item.get("current_version"));
        list.push(p);
    }
    list
}

/// `brew search --formula <q>`: one name per line. Installed entries carry a
/// check mark.
pub fn parse_search(out: &str) -> Vec<Package> {
    lines(out)
        .map(|l| l.trim().trim_end_matches('✔').trim())
        .filter(|l| !l.starts_with("==>") && !l.contains(' '))
        .filter(|l| !l.is_empty())
        .map(|name| Package::new(ManagerId::Brew, name))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formulae() {
        let leaves = "gh\nnode@20\nhomebrew/core/wget\n";
        let versions = "gh 2.62.0\nnode@20 20.18.1\nwget 1.24.5\nopenssl@3 3.4.0\n";
        let list = formulae_from(leaves, versions);
        let names: Vec<_> = list.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, vec!["gh", "node@20", "wget"]);
        assert_eq!(list[2].id, "homebrew/core/wget");
        assert_eq!(list[2].version, "1.24.5");
    }

    #[test]
    fn outdated() {
        let out = r#"{"formulae":[{"name":"gh","installed_versions":["2.61.0"],"current_version":"2.62.0","pinned":false},
                                   {"name":"old","installed_versions":"1.0","current_version":"2.0"}],
                      "casks":[]}"#;
        let list = parse_outdated(out);
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].version, "2.61.0");
        assert_eq!(list[0].new_version, "2.62.0");
        assert_eq!(list[1].version, "1.0");
        assert!(parse_outdated("not json").is_empty());
    }

    #[test]
    fn search() {
        let out = "==> Formulae\nwget ✔\nwget2\n\n";
        let list = parse_search(out);
        let names: Vec<_> = list.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(names, vec!["wget", "wget2"]);
        assert!(parse_search("No formulae or casks found for \"zz\".").is_empty());
    }
}
