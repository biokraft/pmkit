use crate::doctor::runner::Runner;
use crate::error::PmError;
use std::path::Path;

/// Where the team's code is hosted. Decides which pull-request CLI the doctor
/// probes and the preamble names. Closed on purpose: a new host is a new
/// variant plus a probe, nothing more.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Forge {
    GitHub,
    Bitbucket,
    Both,
}

impl Forge {
    pub fn all() -> [Forge; 3] {
        [Forge::GitHub, Forge::Bitbucket, Forge::Both]
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Forge::GitHub => "github",
            Forge::Bitbucket => "bitbucket",
            Forge::Both => "both",
        }
    }

    /// Shown in the wizard's picker.
    pub fn label(self) -> &'static str {
        match self {
            Forge::GitHub => "GitHub",
            Forge::Bitbucket => "Bitbucket Cloud",
            Forge::Both => "Both",
        }
    }

    pub fn includes_github(self) -> bool {
        matches!(self, Forge::GitHub | Forge::Both)
    }

    pub fn includes_bitbucket(self) -> bool {
        matches!(self, Forge::Bitbucket | Forge::Both)
    }
}

impl std::str::FromStr for Forge {
    type Err = PmError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Forge::all()
            .into_iter()
            .find(|f| f.as_str() == s)
            .ok_or_else(|| {
                PmError::Config(format!(
                    "unknown forge `{s}` — expected one of: {}",
                    Forge::all()
                        .iter()
                        .map(|f| f.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ))
            })
    }
}

/// Guesses the forge from the project's git remotes. `github.com` means
/// GitHub, `bitbucket.org` means Bitbucket Cloud, both hosts present means
/// Both. Returns `None` when there is no git, no remote, or neither host,
/// so the caller can fall back to a default rather than pmkit guessing.
pub fn detect_forge(dir: &Path, r: &dyn Runner) -> Option<Forge> {
    let dir = dir.to_string_lossy();
    let out = r.run("git", &["-C", &dir, "remote", "-v"]);
    if !out.ok() {
        return None;
    }
    let text = out.stdout.to_lowercase();
    let github = text.contains("github.com");
    let bitbucket = text.contains("bitbucket.org");
    match (github, bitbucket) {
        (true, true) => Some(Forge::Both),
        (true, false) => Some(Forge::GitHub),
        (false, true) => Some(Forge::Bitbucket),
        (false, false) => None,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::doctor::runner::FakeRunner;
    use std::path::Path;

    #[test]
    fn every_forge_round_trips_through_its_string() {
        for f in Forge::all() {
            assert_eq!(f.as_str().parse::<Forge>().ok(), Some(f), "{}", f.as_str());
        }
    }

    #[test]
    fn an_unknown_forge_is_rejected_and_lists_the_valid_ones() {
        let err = "gitlab".parse::<Forge>().unwrap_err().to_string();
        assert!(err.contains("github, bitbucket, both"), "{err}");
    }

    #[test]
    fn both_includes_each_host_and_the_singles_exclude_the_other() {
        assert!(Forge::Both.includes_github() && Forge::Both.includes_bitbucket());
        assert!(Forge::GitHub.includes_github() && !Forge::GitHub.includes_bitbucket());
        assert!(Forge::Bitbucket.includes_bitbucket() && !Forge::Bitbucket.includes_github());
    }

    #[test]
    fn a_github_remote_is_detected() {
        let r = FakeRunner::new().with(
            "git",
            0,
            "origin\tgit@github.com:biokraft/pmkit.git (fetch)\norigin\tgit@github.com:biokraft/pmkit.git (push)\n",
        );
        assert_eq!(detect_forge(Path::new("/p"), &r), Some(Forge::GitHub));
    }

    #[test]
    fn a_bitbucket_remote_is_detected() {
        let r = FakeRunner::new().with(
            "git",
            0,
            "origin\thttps://bitbucket.org/acme/api.git (fetch)\n",
        );
        assert_eq!(detect_forge(Path::new("/p"), &r), Some(Forge::Bitbucket));
    }

    #[test]
    fn mixed_remotes_are_detected_as_both() {
        let r = FakeRunner::new().with(
            "git",
            0,
            "origin\tgit@bitbucket.org:acme/api.git (fetch)\nmirror\thttps://github.com/acme/api.git (fetch)\n",
        );
        assert_eq!(detect_forge(Path::new("/p"), &r), Some(Forge::Both));
    }

    #[test]
    fn no_git_no_remote_or_an_unknown_host_is_none() {
        assert_eq!(detect_forge(Path::new("/p"), &FakeRunner::new()), None);
        let empty = FakeRunner::new().with("git", 0, "");
        assert_eq!(detect_forge(Path::new("/p"), &empty), None);
        let gitlab = FakeRunner::new().with("git", 0, "origin\tgit@gitlab.com:x/y.git (fetch)\n");
        assert_eq!(detect_forge(Path::new("/p"), &gitlab), None);
    }
}
