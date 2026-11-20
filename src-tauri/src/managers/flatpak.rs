//! Flatpak. Commands are run with `--columns=...`; when the output is not a
//! terminal, flatpak separates the columns with tabs.

use super::text::lines;
use crate::model::{ManagerId, Package};

fn cols(line: &str) -> Vec<String> {
    line.split('\t').map(|s| s.trim().to_string()).collect()
}

/// `flatpak list --app --columns=application,name,version,origin,installation`
pub fn parse_list(out: &str) -> Vec<Package> {
    lines(out)
        .filter_map(|l| {
            let c = cols(l);
            if c.len() < 5 || c[0].is_empty() {
                return None;
            }
            let mut p = Package::new(ManagerId::Flatpak, &c[0]);
            if !c[1].is_empty() {
                p.name = c[1].clone();
            }
            p.version = c[2].clone();
            p.origin = c[3].clone();
            p.scope = c[4].clone();
            Some(p)
        })
        .collect()
}

/// `flatpak remote-ls --updates --app --columns=application,name,version,origin`
pub fn parse_updates(out: &str, scope: &str) -> Vec<Package> {
    lines(out)
        .filter_map(|l| {
            let c = cols(l);
            if c.len() < 4 || c[0].is_empty() {
                return None;
            }
            let mut p = Package::new(ManagerId::Flatpak, &c[0]);
            if !c[1].is_empty() {
                p.name = c[1].clone();
            }
            p.new_version = c[2].clone();
            p.origin = c[3].clone();
            p.scope = scope.to_string();
            Some(p)
        })
        .collect()
}

/// `flatpak search --columns=application,name,version,remotes,description <q>`
pub fn parse_search(out: &str) -> Vec<Package> {
    if out.trim_start().starts_with("No matches") {
        return Vec::new();
    }
    lines(out)
        .filter_map(|l| {
            let c = cols(l);
            if c.len() < 5 || c[0].is_empty() {
                return None;
            }
            let mut p = Package::new(ManagerId::Flatpak, &c[0]);
            if !c[1].is_empty() {
                p.name = c[1].clone();
            }
            p.version = c[2].clone();
            p.origin = c[3].split(',').next().unwrap_or("").trim().to_string();
            p.description = c[4].clone();
            Some(p)
        })
        .collect()
}

/// `flatpak remotes --columns=name,options`: which installation a remote
/// belongs to ("system" or "user").
pub fn parse_remotes(out: &str) -> Vec<(String, String)> {
    lines(out)
        .filter_map(|l| {
            let c = cols(l);
            let name = c.first()?.clone();
            let options = c.get(1).cloned().unwrap_or_default();
            let scope = if options.split(',').any(|o| o.trim() == "user") {
                "user"
            } else {
                "system"
            };
            Some((name, scope.to_string()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list() {
        let out = "org.mozilla.firefox\tFirefox\t133.0\tflathub\tsystem\n\
                   com.spotify.Client\tSpotify\t1.2.50\tflathub\tuser\n";
        let list = parse_list(out);
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].name, "Firefox");
        assert_eq!(list[1].scope, "user");
    }

    #[test]
    fn updates_and_search() {
        let u = parse_updates(
            "org.gimp.GIMP\tGNU Image Manipulation Program\t3.0.2\tflathub\n",
            "system",
        );
        assert_eq!(u[0].new_version, "3.0.2");
        assert_eq!(u[0].scope, "system");

        assert!(parse_search("No matches found\n").is_empty());
        let s = parse_search(
            "org.gimp.GIMP\tGIMP\t3.0.2\tflathub,fedora\tCreate images and edit photographs\n",
        );
        assert_eq!(s[0].origin, "flathub");
        assert_eq!(s[0].description, "Create images and edit photographs");
    }

    #[test]
    fn remotes() {
        let r = parse_remotes("flathub\tsystem\nmine\tuser\nother\t\n");
        assert_eq!(r[0], ("flathub".into(), "system".into()));
        assert_eq!(r[1], ("mine".into(), "user".into()));
        assert_eq!(r[2].1, "system");
    }
}
