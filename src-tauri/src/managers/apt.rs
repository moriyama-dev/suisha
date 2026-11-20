//! apt (Debian / Ubuntu).

use std::collections::{HashMap, HashSet};

use super::text::lines;
use crate::model::{ManagerId, Package};

/// `apt-mark showmanual`: one name per line, sometimes with `:arch`.
pub fn parse_showmanual(out: &str) -> HashSet<String> {
    lines(out)
        .map(|l| l.trim().split(':').next().unwrap_or("").to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

/// The `dpkg-query` format [`parse_dpkg_query`] reads.
pub const DPKG_FORMAT: &str =
    "${Package}\t${Version}\t${db:Status-Abbrev}\t${Essential}\t${Priority}\t${binary:Summary}\n";

/// Installed packages, without the base system (essential packages and
/// those with priority "required" or "important"), which must not be removed.
pub fn parse_dpkg_query(out: &str) -> HashMap<String, (String, String)> {
    let mut map = HashMap::new();
    for line in lines(out) {
        let mut parts = line.splitn(6, '\t');
        let (Some(name), Some(version), Some(status), Some(essential), Some(priority)) = (
            parts.next(),
            parts.next(),
            parts.next(),
            parts.next(),
            parts.next(),
        ) else {
            continue;
        };
        if !status.starts_with("ii")
            || essential.trim() == "yes"
            || matches!(priority.trim(), "required" | "important")
        {
            continue;
        }
        let summary = parts.next().unwrap_or("").trim().to_string();
        map.insert(name.to_string(), (version.to_string(), summary));
    }
    map
}

/// Manually installed packages that are still installed.
pub fn installed_from(showmanual: &str, dpkg: &str) -> Vec<Package> {
    let manual = parse_showmanual(showmanual);
    let info = parse_dpkg_query(dpkg);
    let mut list: Vec<Package> = manual
        .into_iter()
        .filter_map(|name| {
            let (version, summary) = info.get(&name)?;
            let mut p = Package::new(ManagerId::Apt, &name);
            p.version = version.clone();
            p.description = summary.clone();
            Some(p)
        })
        .collect();
    list.sort_by(|a, b| a.id.cmp(&b.id));
    list
}

/// `apt list --upgradable`:
/// `distro-info/noble-updates 1.7ubuntu0.1 amd64 [upgradable from: 1.7build1]`
pub fn parse_upgradable(out: &str) -> Vec<Package> {
    let mut list = Vec::new();
    for line in lines(out) {
        let Some((name, rest)) = line.split_once('/') else {
            continue;
        };
        let mut words = rest.split_whitespace();
        let _suite = words.next();
        let Some(new_version) = words.next() else {
            continue;
        };
        let old = line
            .split_once("[upgradable from: ")
            .and_then(|(_, r)| r.strip_suffix(']'))
            .unwrap_or("");
        let mut p = Package::new(ManagerId::Apt, name.trim());
        p.new_version = new_version.to_string();
        p.version = old.to_string();
        list.push(p);
    }
    list
}

/// From `apt-get -s upgrade --with-new-pkgs`: updates apt will not install
/// yet. Ubuntu releases some updates in stages ("phased updates"), and apt
/// holds them back until this computer's turn comes.
pub fn parse_held(simulation: &str) -> HashMap<String, String> {
    let mut held = HashMap::new();
    let mut reason: Option<&str> = None;
    for line in simulation.lines() {
        if line.starts_with(' ') {
            if let Some(r) = reason {
                for name in line.split_whitespace() {
                    held.insert(
                        name.split(':').next().unwrap_or(name).to_string(),
                        r.to_string(),
                    );
                }
            }
            continue;
        }
        reason = if line.starts_with("The following upgrades have been deferred due to phasing") {
            Some("phased")
        } else if line.starts_with("The following packages have been kept back") {
            Some("kept-back")
        } else {
            None
        };
    }
    held
}

/// `apt-cache search --names-only <q>`: `name - description`
pub fn parse_search(out: &str) -> Vec<Package> {
    lines(out)
        .filter_map(|l| {
            let (name, desc) = l.split_once(" - ")?;
            let mut p = Package::new(ManagerId::Apt, name.trim());
            p.description = desc.trim().to_string();
            Some(p)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installed_is_manual_and_installed() {
        let manual = "git\nvim\nlibfoo:i386\nremoved-pkg\nbash\napt\n";
        let dpkg = "git\t1:2.43.0-1ubuntu7\tii \tno\toptional\tfast, scalable, distributed revision control system\n\
                    vim\t2:9.1.0016-1ubuntu7\tii \t\toptional\tVi IMproved - enhanced vi editor\n\
                    libfoo\t1.0\tii \t\toptional\tfoo library\n\
                    removed-pkg\t1.0\trc \t\toptional\tgone\n\
                    curl\t8.5.0\tii \t\toptional\tcommand line tool\n\
                    bash\t5.2.21-2ubuntu4\tii \tyes\trequired\tGNU Bourne Again SHell\n\
                    apt\t2.8.3\tii \t\timportant\tcommandline package manager\n";
        let list = installed_from(manual, dpkg);
        let names: Vec<_> = list.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(names, vec!["git", "libfoo", "vim"]);
        assert_eq!(list[0].version, "1:2.43.0-1ubuntu7");
        assert_eq!(list[2].description, "Vi IMproved - enhanced vi editor");
    }

    #[test]
    fn upgradable() {
        let out = "Listing...\n\
            alsa-ucm-conf/noble-updates 1.2.10-1ubuntu5.16 all [upgradable from: 1.2.10-1ubuntu5.15]\n\
            distro-info/noble-updates 1.7ubuntu0.1 amd64 [upgradable from: 1.7build1]\n";
        let list = parse_upgradable(out);
        assert_eq!(list.len(), 2);
        assert_eq!(list[1].id, "distro-info");
        assert_eq!(list[1].version, "1.7build1");
        assert_eq!(list[1].new_version, "1.7ubuntu0.1");
    }

    #[test]
    fn held_updates() {
        let sim = "NOTE: This is only a simulation!\n\
            Calculating upgrade...\n\
            The following NEW packages will be installed:\n\
            \x20 linux-headers-6.8.0-151\n\
            The following upgrades have been deferred due to phasing:\n\
            \x20 libnss-systemd libpam-systemd\n\
            \x20 udev\n\
            The following packages have been kept back:\n\
            \x20 firefox:amd64\n\
            The following packages will be upgraded:\n\
            \x20 gh tailscale\n\
            2 upgraded, 1 newly installed, 0 to remove and 4 not upgraded.\n";
        let held = parse_held(sim);
        assert_eq!(held.len(), 4);
        assert_eq!(held["udev"], "phased");
        assert_eq!(held["firefox"], "kept-back");
        assert!(!held.contains_key("gh"));
        assert!(!held.contains_key("linux-headers-6.8.0-151"));
    }

    #[test]
    fn search() {
        let out = "htop - interactive processes viewer\nbtop - Modern and colorful command line resource monitor\n";
        let list = parse_search(out);
        assert_eq!(list[0].id, "htop");
        assert_eq!(
            list[1].description,
            "Modern and colorful command line resource monitor"
        );
    }
}
