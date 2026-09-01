#![allow(clippy::unwrap_used)]
use pmkit::skills::SKILLS;

/// Returns the value of a top-level frontmatter key, or None.
fn frontmatter(content: &str, key: &str) -> Option<String> {
    let body = content.strip_prefix("---\n")?;
    let end = body.find("\n---")?;
    for line in body[..end].lines() {
        if let Some(rest) = line.strip_prefix(&format!("{key}:")) {
            return Some(rest.trim().to_string());
        }
    }
    None
}

#[test]
fn every_skill_has_frontmatter_whose_name_matches_the_registry() {
    for skill in SKILLS.iter() {
        let name = frontmatter(skill.content, "name")
            .unwrap_or_else(|| panic!("{} has no frontmatter name", skill.name));
        assert_eq!(name, skill.name, "frontmatter name must match directory");
        let description = frontmatter(skill.content, "description")
            .unwrap_or_else(|| panic!("{} has no frontmatter description", skill.name));
        assert!(
            description.len() > 40,
            "{}: description must say when to use the skill",
            skill.name
        );
    }
}

#[test]
fn every_skill_name_uses_the_pmk_prefix_and_is_unique() {
    let mut seen = std::collections::BTreeSet::new();
    for skill in SKILLS.iter() {
        assert!(skill.name.starts_with("pmk-"), "{}", skill.name);
        assert!(seen.insert(skill.name), "duplicate skill {}", skill.name);
        assert!(!skill.summary.is_empty());
        assert!(
            skill.summary.len() <= 72,
            "{} summary too long for a narrow terminal",
            skill.name
        );
    }
}

#[test]
fn the_three_hard_gates_appear_in_the_loop_skill() {
    let loop_skill = SKILLS
        .iter()
        .find(|s| s.name == "pmk-feature-loop")
        .unwrap();
    for phrase in [
        "git push",
        "Jira",
        "explicit yes",
        // Each command needs its own approval — the gate is per-invocation, not per-session.
        "Each command needs its own yes",
        // Routine dependency installs must not be swept into the egress gate, or the gate
        // gets ignored wholesale. Kept short enough to survive a wrapped markdown line.
        "is not this gate; sending data somewhere is",
    ] {
        assert!(
            loop_skill.content.contains(phrase),
            "missing gate text: {phrase}"
        );
    }
}
