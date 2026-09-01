use crate::capabilities::Capabilities;
use crate::emit::plan_files;
use crate::error::Result;
use crate::state::{
    apply, load_state, save_state, uninstall, Entry, FileState, MissingPolicy, Outcome,
};
use crate::target::{destination_for, Target};
use std::path::Path;

/// Shared machinery for `install` and `refresh`. The only difference between
/// the two is what happens to a tracked file that has been deleted from
/// disk: an explicit `install` restores it, because a human just asked for
/// these targets; `refresh` must not, because it runs unattended (or at
/// least without the user naming targets) and a deliberately deleted file
/// silently reappearing there would defeat the point of deleting it. That is
/// exactly the distinction `MissingPolicy` exists to draw — see its doc
/// comment in `state.rs`.
fn install_with_policy(
    targets: &[Target],
    project_dir: &Path,
    home: &Path,
    caps: &Capabilities,
    state_file: &Path,
    policy: MissingPolicy,
) -> Result<Vec<Outcome>> {
    let (mut entries, warning) = load_state(state_file);
    if let Some(w) = warning {
        eprintln!("warning: {w}");
    }
    let mut out = Vec::new();
    for &t in targets {
        let dest = destination_for(t, project_dir, home);
        let files = plan_files(t, caps, &dest);
        out.extend(apply(&files, t, &mut entries, policy));
    }
    save_state(state_file, &entries)?;
    Ok(out)
}

pub fn install(
    targets: &[Target],
    project_dir: &Path,
    home: &Path,
    caps: &Capabilities,
    state_file: &Path,
) -> Result<Vec<Outcome>> {
    install_with_policy(
        targets,
        project_dir,
        home,
        caps,
        state_file,
        MissingPolicy::Restore,
    )
}

/// Re-emits every target that already has at least one tracked file, without
/// resurrecting anything the user deliberately deleted. `install`'s
/// `MissingPolicy::Restore` is right for an explicit, human-directed
/// `skill install --target x`: the user just asked pmkit to put those files
/// there. `refresh` is not that — it re-applies whatever is already tracked,
/// so a `Missing` tracked file here is far more likely to be a deliberate
/// deletion than a fresh request, and `Restore` would silently undo it. Using
/// `MissingPolicy::Preserve` keeps that deletion: the file stays gone and its
/// entry is pruned from state instead of being rewritten.
pub fn refresh(
    project_dir: &Path,
    home: &Path,
    caps: &Capabilities,
    state_file: &Path,
) -> Result<Vec<Outcome>> {
    let (entries, _) = load_state(state_file);
    let targets: Vec<Target> = Target::all()
        .into_iter()
        .filter(|t| entries.iter().any(|e| e.target == t.as_str()))
        .collect();
    install_with_policy(
        &targets,
        project_dir,
        home,
        caps,
        state_file,
        MissingPolicy::Preserve,
    )
}

pub fn list(state_file: &Path) -> Result<Vec<(Entry, FileState)>> {
    let (entries, _) = load_state(state_file);
    Ok(entries
        .into_iter()
        .map(|e| {
            let state = match std::fs::read(&e.path) {
                Err(_) => FileState::Missing,
                Ok(bytes) => {
                    if crate::state::content_hash(&bytes) == e.sha256 {
                        FileState::Current
                    } else {
                        FileState::Modified
                    }
                }
            };
            (e, state)
        })
        .collect())
}

pub fn remove(target: Option<Target>, state_file: &Path) -> Result<Vec<Outcome>> {
    let (mut entries, _) = load_state(state_file);
    let out = uninstall(&mut entries, target);
    save_state(state_file, &entries)?;
    Ok(out)
}
