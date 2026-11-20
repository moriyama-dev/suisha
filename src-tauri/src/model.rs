use serde::{Deserialize, Serialize};

/// The package managers Suisha knows how to drive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum ManagerId {
    Apt,
    Snap,
    Flatpak,
    Brew,
}

impl ManagerId {
    pub const ALL: [ManagerId; 4] = [
        ManagerId::Apt,
        ManagerId::Snap,
        ManagerId::Flatpak,
        ManagerId::Brew,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            ManagerId::Apt => "apt",
            ManagerId::Snap => "snap",
            ManagerId::Flatpak => "flatpak",
            ManagerId::Brew => "brew",
        }
    }

    /// Whether changing packages needs administrator rights.
    pub fn needs_admin(self) -> bool {
        matches!(self, ManagerId::Apt | ManagerId::Snap)
    }
}

/// One package as shown in a list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Package {
    pub manager: Option<ManagerId>,
    /// The identifier passed back to the package manager.
    pub id: String,
    /// A human-friendly name. Falls back to `id`.
    pub name: String,
    #[serde(default)]
    pub version: String,
    /// The version an update would bring, when known.
    #[serde(default)]
    pub new_version: String,
    #[serde(default)]
    pub description: String,
    /// Flatpak: "user" or "system". Empty for other managers.
    #[serde(default)]
    pub scope: String,
    /// Flatpak: the remote the app comes from.
    #[serde(default)]
    pub origin: String,
    /// Snap: whether the snap needs `--classic` confinement.
    #[serde(default)]
    pub classic: bool,
    /// apt: why an update cannot be installed yet ("phased" or "kept-back").
    #[serde(default)]
    pub held: String,
}

impl Package {
    pub fn new(manager: ManagerId, id: impl Into<String>) -> Self {
        let id = id.into();
        Package {
            manager: Some(manager),
            name: id.clone(),
            id,
            ..Default::default()
        }
    }
}

/// A reference to a package that an operation should act on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageRef {
    pub manager: ManagerId,
    pub id: String,
    #[serde(default)]
    pub scope: String,
    #[serde(default)]
    pub origin: String,
    #[serde(default)]
    pub classic: bool,
}

impl From<&Package> for PackageRef {
    fn from(p: &Package) -> Self {
        PackageRef {
            manager: p.manager.unwrap_or(ManagerId::Apt),
            id: p.id.clone(),
            scope: p.scope.clone(),
            origin: p.origin.clone(),
            classic: p.classic,
        }
    }
}

/// Accept only identifiers that look like package names.
///
/// The value is passed as a separate argument (never through a shell), and it
/// must not start with `-` so that it can never be read as an option.
pub fn valid_package_id(id: &str) -> bool {
    let mut chars = id.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    id.len() <= 200
        && first.is_ascii_alphanumeric()
        && chars.all(|c| c.is_ascii_alphanumeric() || "._+-:@/".contains(c))
}

/// Flatpak remotes and scopes use a narrower alphabet.
pub fn valid_simple_word(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 100
        && s.chars().next().is_some_and(|c| c.is_ascii_alphanumeric())
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || "._-".contains(c))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_ids() {
        for ok in [
            "git",
            "libc6",
            "g++",
            "python3.12",
            "org.mozilla.firefox",
            "someone/tap/tool",
            "node@20",
            "libfoo:amd64",
        ] {
            assert!(valid_package_id(ok), "{ok}");
        }
        for bad in [
            "", "-y", "--purge", "a b", "a;rm", "$(x)", "a\nb", "名前", "`x`", "a|b",
        ] {
            assert!(!valid_package_id(bad), "{bad:?}");
        }
    }

    #[test]
    fn simple_words() {
        assert!(valid_simple_word("flathub"));
        assert!(valid_simple_word("user"));
        assert!(!valid_simple_word("--user"));
        assert!(!valid_simple_word("a/b"));
    }
}
