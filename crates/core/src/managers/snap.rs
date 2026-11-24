//! snap (Ubuntu).

use super::text::{lines, split_columns};
use crate::model::{ManagerId, Package};

/// Snaps that only exist to support other snaps. They are hidden from the
/// lists so that nobody removes them by accident, and not exported.
pub fn is_support_snap(name: &str, notes: &str) -> bool {
    let typed = notes
        .split(',')
        .any(|n| matches!(n.trim(), "base" | "core" | "snapd" | "gadget" | "kernel"));
    typed
        || name == "bare"
        || name == "gtk-common-themes"
        || name.starts_with("gnome-")
        || name.starts_with("kf5-")
        || name.starts_with("kf6-")
        || name.starts_with("kde-frameworks-")
        || name.starts_with("mesa-")
        || name.starts_with("core")
}

/// `snap list`:
/// `Name  Version  Rev  Tracking  Publisher  Notes`
pub fn parse_list(out: &str) -> Vec<Package> {
    let mut list = Vec::new();
    for line in lines(out).skip(1) {
        let Some(cols) = split_columns(line, 6) else {
            continue;
        };
        let notes = cols[5].as_str();
        if is_support_snap(&cols[0], notes) {
            continue;
        }
        let mut p = Package::new(ManagerId::Snap, &cols[0]);
        p.version = cols[1].clone();
        p.classic = notes.split(',').any(|n| n.trim() == "classic");
        list.push(p);
    }
    list
}

/// `snap refresh --list`:
/// `Name  Version  Rev  Size  Publisher  Notes`, or "All snaps up to date."
pub fn parse_refresh_list(out: &str) -> Vec<Package> {
    if !out.trim_start().starts_with("Name") {
        return Vec::new();
    }
    let mut list = Vec::new();
    for line in lines(out).skip(1) {
        let Some(cols) = split_columns(line, 6) else {
            continue;
        };
        if is_support_snap(&cols[0], &cols[5]) {
            continue;
        }
        let mut p = Package::new(ManagerId::Snap, &cols[0]);
        p.new_version = cols[1].clone();
        list.push(p);
    }
    list
}

/// `snap find <q>`:
/// `Name  Version  Publisher  Notes  Summary`
pub fn parse_find(out: &str) -> Vec<Package> {
    if !out.trim_start().starts_with("Name") {
        return Vec::new();
    }
    let mut list = Vec::new();
    for line in lines(out).skip(1) {
        let Some(cols) = split_columns(line, 5) else {
            continue;
        };
        let mut p = Package::new(ManagerId::Snap, &cols[0]);
        p.version = cols[1].clone();
        p.classic = cols[3].split(',').any(|n| n.trim() == "classic");
        p.description = cols[4].clone();
        list.push(p);
    }
    list
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_hides_support_snaps() {
        let out = "Name               Version          Rev    Tracking         Publisher     Notes\n\
            bare               1.0              5      latest/stable    canonical✓    base\n\
            code               1.95.3           176    latest/stable    vscode✓       classic\n\
            core22             20241001         1663   latest/stable    canonical✓    base\n\
            firefox            132.0.2-1        5273   latest/stable/…  mozilla✓      -\n\
            gnome-42-2204      0+git.510a601    176    latest/stable    canonical✓    -\n\
            gtk-common-themes  0.1-81-g442e511  1535   latest/stable    canonical✓    -\n\
            snapd              2.66.1           23258  latest/stable    canonical✓    snapd\n";
        let list = parse_list(out);
        let names: Vec<_> = list.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(names, vec!["code", "firefox"]);
        assert!(list[0].classic);
        assert_eq!(list[1].version, "132.0.2-1");
    }

    #[test]
    fn refresh_list() {
        assert!(parse_refresh_list("All snaps up to date.\n").is_empty());
        let out = "Name     Version    Rev   Size   Publisher   Notes\n\
                   firefox  133.0-1    5357  284MB  mozilla✓    -\n";
        let list = parse_refresh_list(out);
        assert_eq!(list[0].id, "firefox");
        assert_eq!(list[0].new_version, "133.0-1");
    }

    #[test]
    fn find() {
        assert!(parse_find("No matching snaps for \"zzz\"\n").is_empty());
        let out = "Name    Version  Publisher   Notes    Summary\n\
                   vlc     3.0.20   videolan✓   -        The ultimate media player\n\
                   code    abc123   vscode✓     classic  Code editing. Redefined.\n";
        let list = parse_find(out);
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].description, "The ultimate media player");
        assert!(list[1].classic);
    }
}
