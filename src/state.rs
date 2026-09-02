use crate::emit::{EmitFile, FileKind};
use crate::error::{PmError, Result};
use crate::skills::skill_by_name;
use crate::target::Target;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub fn content_hash(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn default_created() -> bool {
    true
}

/// One tracked file. `sha256` is what pmkit itself wrote, which is how a local
/// edit is detected: bytes matching neither the wanted content nor this hash
/// were written by someone else and are left alone. `created` records that
/// pmkit put the file there at all — a hash can prove the bytes match, never
/// who wrote them — so uninstall removes only pmkit's own files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub path: PathBuf,
    pub target: String,
    pub kind: String,
    pub sha256: String,
    pub version: String,
    pub skill: String,
    #[serde(default = "default_created")]
    pub created: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Installed,
    Refreshed,
    Unchanged,
    SkippedModified,
    Pruned,
    Failed,
}

impl Action {
    pub fn as_str(self) -> &'static str {
        match self {
            Action::Installed => "installed",
            Action::Refreshed => "refreshed",
            Action::Unchanged => "unchanged",
            Action::SkippedModified => "skipped (you edited it)",
            Action::Pruned => "pruned",
            Action::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileState {
    Current,
    Stale,
    Modified,
    Missing,
}

impl FileState {
    pub fn as_str(self) -> &'static str {
        match self {
            FileState::Current => "current",
            FileState::Stale => "stale",
            FileState::Modified => "modified",
            FileState::Missing => "missing",
        }
    }
}

/// Whether a tracked file that has been deleted should be written back.
/// An explicit `pmkit skill install` passes `Restore`, because a human asked
/// for it. Anything automatic passes `Preserve`, because a deliberately deleted
/// file must not silently reappear.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissingPolicy {
    Restore,
    Preserve,
}

#[derive(Debug, Clone)]
pub struct Outcome {
    pub path: PathBuf,
    pub target: String,
    pub action: Action,
}

pub fn state_path() -> PathBuf {
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        if !xdg.is_empty() {
            return PathBuf::from(xdg).join("pmkit").join("skills.json");
        }
    }
    let home = std::env::var_os("HOME").unwrap_or_default();
    PathBuf::from(home)
        .join(".config")
        .join("pmkit")
        .join("skills.json")
}

/// A missing state file means nothing is tracked. A corrupt one is reported but
/// treated as empty, so a hand-edited file cannot brick the tool.
pub fn load_state(path: &Path) -> (Vec<Entry>, Option<String>) {
    let raw = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return (Vec::new(), None),
        Err(err) => {
            return (
                Vec::new(),
                Some(format!("could not read {}: {err}", path.display())),
            )
        }
    };
    if raw.trim().is_empty() {
        return (Vec::new(), None);
    }
    match serde_json::from_str::<Vec<Entry>>(&raw) {
        Ok(entries) => (entries, None),
        Err(err) => (
            Vec::new(),
            Some(format!("ignoring unreadable {}: {err}", path.display())),
        ),
    }
}

/// Writes via a temp file plus `rename`, so two pmkit processes racing right
/// after an upgrade cannot leave a truncated state file.
pub fn save_state(path: &Path, entries: &[Entry]) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| PmError::Config(format!("state path {} has no parent", path.display())))?;
    std::fs::create_dir_all(parent)?;
    let json = serde_json::to_string_pretty(entries)?;
    let tmp = parent.join(format!(".skills.json.tmp.{}", std::process::id()));
    std::fs::write(&tmp, json)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

/// Refuses any path that is not one pmkit writes. Every write and every removal
/// driven by a state entry goes through this first: the state file is
/// user-editable, and nothing it names should let pmkit touch an arbitrary path.
pub fn is_pmkit_path(path: &Path) -> bool {
    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    if matches!(
        name.as_str(),
        "AGENTS.md"
            | "settings.json"
            | "hooks.json"
            | "README.md"
            | "openai.yaml"
            | "pmkit-chatgpt-instructions.md"
    ) {
        return true;
    }
    if name != "SKILL.md" {
        return false;
    }
    path.parent()
        .and_then(|p| p.file_name())
        .map(|d| skill_by_name(&d.to_string_lossy()).is_some())
        .unwrap_or(false)
}

fn kind_str(kind: FileKind) -> &'static str {
    match kind {
        FileKind::Skill => "skill",
        FileKind::Config => "config",
        FileKind::Instructions => "instructions",
    }
}

fn state_of(path: &Path, wanted: &str, tracked: Option<&Entry>) -> FileState {
    match std::fs::read(path) {
        Err(_) => FileState::Missing,
        Ok(bytes) => {
            let actual = content_hash(&bytes);
            if actual == wanted {
                FileState::Current
            } else if tracked.map(|e| e.sha256 == actual).unwrap_or(false) {
                FileState::Stale
            } else {
                FileState::Modified
            }
        }
    }
}

/// Writes the planned files, respecting local edits, and updates `entries` in
/// place. Never touches a path `is_pmkit_path` rejects.
///
/// `FileState::Modified` means the bytes on disk match neither the content
/// pmkit wants nor any hash pmkit previously recorded for this path — i.e.
/// pmkit cannot prove it wrote them. That is true whether or not the path
/// happens to be tracked yet (a plan can name a path nothing has tracked
/// before, and unrelated content can already be sitting there), so the
/// `SkippedModified` guard does not require `tracked.is_some()`: an untracked
/// foreign file must be left alone exactly like a tracked, locally-edited one.
pub fn apply(
    files: &[EmitFile],
    target: Target,
    entries: &mut Vec<Entry>,
    policy: MissingPolicy,
) -> Vec<Outcome> {
    let mut out = Vec::new();
    for f in files {
        if !is_pmkit_path(&f.path) {
            out.push(Outcome {
                path: f.path.clone(),
                target: target.as_str().to_string(),
                action: Action::Failed,
            });
            continue;
        }
        let wanted = content_hash(f.contents.as_bytes());
        let tracked_index = entries.iter().position(|e| e.path == f.path);
        let tracked = tracked_index.map(|i| &entries[i]);
        let state = state_of(&f.path, &wanted, tracked);

        // An entry recorded as `created: false` is pmkit's own record that it did
        // NOT put this file on disk. That must hold on the write path too, not
        // just in `uninstall` — otherwise the flag is meaningless on the one
        // path that can destroy data. Treat such a tracked file as never
        // writable: report it the same way as a foreign edit, and leave both
        // the file and the entry's `created: false` untouched.
        let never_writable = tracked.map(|e| !e.created).unwrap_or(false);

        let action = if never_writable && state != FileState::Current {
            Action::SkippedModified
        } else {
            match state {
                FileState::Current => Action::Unchanged,
                FileState::Modified => Action::SkippedModified,
                FileState::Missing if tracked.is_some() && policy == MissingPolicy::Preserve => {
                    Action::Pruned
                }
                _ => match write_file(&f.path, &f.contents) {
                    Ok(()) if state == FileState::Missing && tracked.is_none() => Action::Installed,
                    Ok(()) => Action::Refreshed,
                    Err(_) => Action::Failed,
                },
            }
        };

        if never_writable {
            // Don't touch the entry at all: no flipping `created` to true, no
            // refreshing its recorded hash.
        } else if matches!(
            action,
            Action::Installed | Action::Refreshed | Action::Unchanged
        ) {
            let entry = Entry {
                path: f.path.clone(),
                target: target.as_str().to_string(),
                kind: kind_str(f.kind).to_string(),
                sha256: wanted,
                version: env!("CARGO_PKG_VERSION").to_string(),
                skill: skill_name_for(f),
                created: true,
            };
            match tracked_index {
                Some(i) => entries[i] = entry,
                None => entries.push(entry),
            }
        }

        out.push(Outcome {
            path: f.path.clone(),
            target: target.as_str().to_string(),
            action,
        });
    }
    out
}

/// The skill a file belongs to, or `-` for shared config.
fn skill_name_for(f: &EmitFile) -> String {
    if f.kind != FileKind::Skill {
        return "-".to_string();
    }
    f.path
        .parent()
        .and_then(|p| p.file_name())
        .map(|d| d.to_string_lossy().to_string())
        .unwrap_or_else(|| "-".to_string())
}

fn write_file(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, contents)?;
    Ok(())
}

/// Removes only files pmkit created and still recognises. Pass `None` to
/// uninstall every target.
pub fn uninstall(entries: &mut Vec<Entry>, target: Option<Target>) -> Vec<Outcome> {
    let mut out = Vec::new();
    let wanted = target.map(|t| t.as_str().to_string());
    entries.retain(|e| {
        if wanted.as_ref().map(|w| &e.target != w).unwrap_or(false) {
            return true;
        }
        if !e.created || !is_pmkit_path(&e.path) {
            return true;
        }
        let action = match std::fs::remove_file(&e.path) {
            Ok(()) => Action::Pruned,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Action::Pruned,
            Err(_) => Action::Failed,
        };
        out.push(Outcome {
            path: e.path.clone(),
            target: e.target.clone(),
            action,
        });
        action == Action::Pruned
    });
    out
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::capabilities::Capabilities;
    use crate::emit::plan_files;
    use crate::target::{Destination, Target};

    fn planned(root: &std::path::Path) -> Vec<crate::emit::EmitFile> {
        let dest = Destination::Repo(root.to_path_buf());
        plan_files(Target::ClaudeCode, &Capabilities::all_present(), &dest)
    }

    #[test]
    fn a_fresh_install_writes_every_file_and_tracks_it() {
        let tmp = tempfile::tempdir().unwrap();
        let files = planned(tmp.path());
        let mut entries = Vec::new();
        let out = apply(
            &files,
            Target::ClaudeCode,
            &mut entries,
            MissingPolicy::Restore,
        );
        assert_eq!(out.len(), files.len());
        assert!(out.iter().all(|o| o.action == Action::Installed));
        assert_eq!(entries.len(), files.len());
        for f in &files {
            assert_eq!(std::fs::read_to_string(&f.path).unwrap(), f.contents);
        }
    }

    #[test]
    fn installing_twice_reports_unchanged_and_does_not_rewrite() {
        let tmp = tempfile::tempdir().unwrap();
        let files = planned(tmp.path());
        let mut entries = Vec::new();
        apply(
            &files,
            Target::ClaudeCode,
            &mut entries,
            MissingPolicy::Restore,
        );
        let out = apply(
            &files,
            Target::ClaudeCode,
            &mut entries,
            MissingPolicy::Restore,
        );
        assert!(out.iter().all(|o| o.action == Action::Unchanged), "{out:?}");
        assert_eq!(entries.len(), files.len());
    }

    #[test]
    fn a_locally_edited_file_is_never_overwritten() {
        let tmp = tempfile::tempdir().unwrap();
        let files = planned(tmp.path());
        let mut entries = Vec::new();
        apply(
            &files,
            Target::ClaudeCode,
            &mut entries,
            MissingPolicy::Restore,
        );
        let victim = &files[0].path;
        std::fs::write(victim, "MY OWN NOTES\n").unwrap();

        // A new pmkit version wants to ship different content.
        let mut changed = files.clone();
        changed[0].contents.push_str("\nnew upstream line\n");
        let out = apply(
            &changed,
            Target::ClaudeCode,
            &mut entries,
            MissingPolicy::Restore,
        );

        assert_eq!(std::fs::read_to_string(victim).unwrap(), "MY OWN NOTES\n");
        assert!(out.iter().any(|o| o.action == Action::SkippedModified));
    }

    #[test]
    fn a_stale_file_is_refreshed_when_the_content_changes() {
        let tmp = tempfile::tempdir().unwrap();
        let files = planned(tmp.path());
        let mut entries = Vec::new();
        apply(
            &files,
            Target::ClaudeCode,
            &mut entries,
            MissingPolicy::Restore,
        );
        let mut changed = files.clone();
        changed[0].contents.push_str("\nnew upstream line\n");
        let out = apply(
            &changed,
            Target::ClaudeCode,
            &mut entries,
            MissingPolicy::Restore,
        );
        assert!(out.iter().any(|o| o.action == Action::Refreshed));
        assert_eq!(
            std::fs::read_to_string(&changed[0].path).unwrap(),
            changed[0].contents
        );
    }

    #[test]
    fn a_deliberately_deleted_file_stays_deleted_under_preserve() {
        let tmp = tempfile::tempdir().unwrap();
        let files = planned(tmp.path());
        let mut entries = Vec::new();
        apply(
            &files,
            Target::ClaudeCode,
            &mut entries,
            MissingPolicy::Restore,
        );
        std::fs::remove_file(&files[0].path).unwrap();
        apply(
            &files,
            Target::ClaudeCode,
            &mut entries,
            MissingPolicy::Preserve,
        );
        assert!(!files[0].path.exists());
        apply(
            &files,
            Target::ClaudeCode,
            &mut entries,
            MissingPolicy::Restore,
        );
        assert!(files[0].path.exists());
    }

    #[test]
    fn uninstall_removes_only_what_pmkit_wrote() {
        let tmp = tempfile::tempdir().unwrap();
        let files = planned(tmp.path());
        let mut entries = Vec::new();
        apply(
            &files,
            Target::ClaudeCode,
            &mut entries,
            MissingPolicy::Restore,
        );
        let bystander = tmp.path().join(".claude").join("mine.md");
        std::fs::write(&bystander, "mine").unwrap();
        entries.push(Entry {
            path: bystander.clone(),
            target: "claude-code".into(),
            kind: "file".into(),
            sha256: content_hash(b"mine"),
            version: "0.0.0".into(),
            skill: "pmk-feature-loop".into(),
            created: false,
        });

        uninstall(&mut entries, Some(Target::ClaudeCode));
        assert!(bystander.exists(), "an untracked-as-ours file must survive");
        assert!(!files[0].path.exists());
    }

    #[test]
    fn state_round_trips_through_json_and_a_corrupt_file_is_treated_as_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("skills.json");
        let entries = vec![Entry {
            path: tmp.path().join("x"),
            target: "codex".into(),
            kind: "file".into(),
            sha256: content_hash(b"x"),
            version: "1.2.3".into(),
            skill: "pmk-jira-flow".into(),
            created: true,
        }];
        save_state(&path, &entries).unwrap();
        let (loaded, warning) = load_state(&path);
        assert_eq!(loaded, entries);
        assert!(warning.is_none());

        std::fs::write(&path, "{not json").unwrap();
        let (loaded, warning) = load_state(&path);
        assert!(loaded.is_empty());
        assert!(warning.is_some());
    }

    #[test]
    fn is_pmkit_path_rejects_arbitrary_paths_from_a_hand_edited_state_file() {
        assert!(is_pmkit_path(std::path::Path::new(
            "/p/.claude/skills/pmk-feature-loop/SKILL.md"
        )));
        assert!(is_pmkit_path(std::path::Path::new("/p/AGENTS.md")));
        assert!(is_pmkit_path(std::path::Path::new("/p/agents/openai.yaml")));
        assert!(!is_pmkit_path(std::path::Path::new("/etc/passwd")));
        assert!(!is_pmkit_path(std::path::Path::new(
            "/p/.claude/skills/not-a-pmkit-skill/SKILL.md"
        )));
    }

    /// Fix round 1, Finding 1: `.cursor/hooks.json` is a real path the Cursor
    /// emitter plans, but before this fix it was silently rejected by the
    /// guard and every install of it failed — the safety gates the hook
    /// enforced were never written to disk.
    #[test]
    fn is_pmkit_path_accepts_cursor_hooks_json_and_still_rejects_arbitrary_paths() {
        assert!(is_pmkit_path(std::path::Path::new("/p/.cursor/hooks.json")));
        assert!(!is_pmkit_path(std::path::Path::new("/etc/passwd")));
    }

    /// Fix round 1, Finding 2: this is the systemic gap — every prior test
    /// exercised `plan_files` (intent) or `apply` against a single target
    /// (behaviour, but not across the board). Parameterised over
    /// `Target::all()` so a sixth target cannot be added without this
    /// coverage, and specifically so a target whose planned file the state
    /// guard silently rejects (as `.cursor/hooks.json` did) cannot pass
    /// unnoticed: every planned path must land on disk with exactly the
    /// planned bytes, and `apply` must never report `Action::Failed`.
    #[test]
    fn every_target_round_trips_every_planned_file_to_disk_with_no_failures() {
        for t in Target::all() {
            let tmp = tempfile::tempdir().unwrap();
            let dest = Destination::Repo(tmp.path().to_path_buf());
            let files = plan_files(t, &Capabilities::all_present(), &dest);
            let mut entries = Vec::new();
            let out = apply(&files, t, &mut entries, MissingPolicy::Restore);

            for o in &out {
                assert_ne!(
                    o.action,
                    Action::Failed,
                    "{}: {} failed to write",
                    t.as_str(),
                    o.path.display()
                );
            }
            for f in &files {
                let on_disk = std::fs::read_to_string(&f.path).unwrap_or_else(|e| {
                    panic!(
                        "{}: {} missing after apply: {e}",
                        t.as_str(),
                        f.path.display()
                    )
                });
                assert_eq!(
                    on_disk,
                    f.contents,
                    "{}: {} landed with the wrong contents",
                    t.as_str(),
                    f.path.display()
                );
            }
        }
    }

    #[test]
    fn apply_never_writes_an_entry_recorded_as_not_created_by_pmkit() {
        // Finding 1: `created: false` is pmkit's own record that it did NOT put
        // this file on disk. `apply` must honor that on the write path (not just
        // `uninstall`), even when the on-disk bytes are Stale relative to new
        // wanted content — a case that would otherwise trigger a rewrite.
        let tmp = tempfile::tempdir().unwrap();
        let files = planned(tmp.path());
        let old_contents = files[0].contents.clone();
        if let Some(parent) = files[0].path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&files[0].path, &old_contents).unwrap();

        let mut entries = vec![Entry {
            path: files[0].path.clone(),
            target: Target::ClaudeCode.as_str().to_string(),
            kind: "skill".into(),
            // Recorded hash matches the bytes on disk, so state resolves to
            // Stale relative to the new wanted content below — the case that
            // would normally trigger a rewrite.
            sha256: content_hash(old_contents.as_bytes()),
            version: "0.0.0".into(),
            skill: "pmk-feature-loop".into(),
            created: false,
        }];

        let mut changed = files.clone();
        changed[0].contents.push_str("\nnew upstream line\n");
        let out = apply(
            &changed,
            Target::ClaudeCode,
            &mut entries,
            MissingPolicy::Restore,
        );

        assert_eq!(
            std::fs::read_to_string(&changed[0].path).unwrap(),
            old_contents,
            "a file pmkit did not create must not be rewritten"
        );
        assert!(out
            .iter()
            .any(|o| o.path == changed[0].path && o.action == Action::SkippedModified));
        assert!(!entries[0].created, "created must not flip to true");
    }

    #[test]
    fn uninstall_leaves_a_not_created_entry_alone_even_when_its_path_would_pass_the_guard() {
        // Finding 2: the bystander must use a filename that legitimately PASSES
        // is_pmkit_path (unlike `.claude/mine.md`), so this test is load-bearing
        // on the `created` check specifically, not riding on the path guard.
        let tmp = tempfile::tempdir().unwrap();
        let bystander = tmp.path().join("AGENTS.md");
        std::fs::write(&bystander, "not pmkit's file\n").unwrap();
        assert!(is_pmkit_path(&bystander), "path must pass the guard");

        let mut entries = vec![Entry {
            path: bystander.clone(),
            target: "claude-code".into(),
            kind: "instructions".into(),
            sha256: content_hash(b"not pmkit's file\n"),
            version: "0.0.0".into(),
            skill: "-".into(),
            created: false,
        }];

        uninstall(&mut entries, Some(Target::ClaudeCode));
        assert!(
            bystander.exists(),
            "an entry recorded as not created by pmkit must survive uninstall"
        );
    }

    #[test]
    fn an_untracked_file_with_foreign_content_is_not_silently_overwritten() {
        // Combination: untracked (no Entry) + present with content that matches
        // neither the wanted bytes nor any recorded hash (there is none). This
        // must be treated the same as a locally modified tracked file: pmkit
        // cannot prove it wrote these bytes, so it must not clobber them.
        let tmp = tempfile::tempdir().unwrap();
        let files = planned(tmp.path());
        let mut entries = Vec::new();
        if let Some(parent) = files[0].path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&files[0].path, "someone else's file, never tracked\n").unwrap();

        let out = apply(
            &files,
            Target::ClaudeCode,
            &mut entries,
            MissingPolicy::Restore,
        );
        assert_eq!(
            std::fs::read_to_string(&files[0].path).unwrap(),
            "someone else's file, never tracked\n"
        );
        assert!(out
            .iter()
            .any(|o| o.path == files[0].path && o.action == Action::SkippedModified));
    }
}
