//! Mac App Store, through the `mas` command line tool.

use super::text::lines;
use crate::model::{ManagerId, Package};

/// Lines look like `497799835  Xcode  (15.0)`, `497799835 Xcode (15.0 -> 15.1)`
/// or, for search results, `  497799835  Xcode  (15.1)`. Some versions of
/// `mas` add a price after the version; it is ignored.
fn parse_line(line: &str) -> Option<(String, String, String)> {
    let line = line.trim();
    let (id, rest) = line.split_once(char::is_whitespace)?;
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let rest = rest.trim();
    let open = rest.rfind('(')?;
    let close = rest[open..].find(')')? + open;
    let name = rest[..open].trim().to_string();
    let version = rest[open + 1..close].trim().to_string();
    if name.is_empty() {
        return None;
    }
    Some((id.to_string(), name, version))
}

pub fn parse_list(out: &str) -> Vec<Package> {
    let mut list: Vec<Package> = lines(out)
        .filter_map(parse_line)
        .map(|(id, name, version)| {
            let mut p = Package::new(ManagerId::Mas, id);
            p.name = name;
            p.version = version;
            p
        })
        .collect();
    list.sort_by_key(|a| a.name.to_lowercase());
    list
}

pub fn parse_outdated(out: &str) -> Vec<Package> {
    lines(out)
        .filter_map(parse_line)
        .map(|(id, name, version)| {
            let mut p = Package::new(ManagerId::Mas, id);
            p.name = name;
            match version.split_once("->") {
                Some((old, new)) => {
                    p.version = old.trim().to_string();
                    p.new_version = new.trim().to_string();
                }
                None => p.new_version = version,
            }
            p
        })
        .collect()
}

pub fn parse_search(out: &str) -> Vec<Package> {
    lines(out)
        .filter_map(parse_line)
        .map(|(id, name, version)| {
            let mut p = Package::new(ManagerId::Mas, id);
            p.name = name;
            p.version = version;
            p
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_outdated_search() {
        let list = parse_list(
            "497799835  Xcode  (15.0)\n409183694  Keynote  (14.2)\nWarning: something\n",
        );
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].name, "Keynote");
        assert_eq!(list[1].id, "497799835");

        let out = parse_outdated("497799835 Xcode (15.0 -> 15.1)\n");
        assert_eq!(out[0].version, "15.0");
        assert_eq!(out[0].new_version, "15.1");

        let s =
            parse_search("   497799835  Xcode (Beta Tools)  (15.1)\n  1  Free App  (2.0)  $0.00\n");
        assert_eq!(s[0].name, "Xcode (Beta Tools)");
        assert_eq!(s[0].version, "15.1");
        assert_eq!(s[1].name, "Free App");
    }
}
