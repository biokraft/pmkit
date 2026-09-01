/// One embedded skill. The text ships *inside* the binary, so every upgrade
/// path — brew, cargo, `pmkit update` — carries new content as an inherent
/// consequence rather than needing a separate sync.
pub struct Skill {
    pub name: &'static str,
    /// One line, shown as this skill's row in the wizard. Kept short enough to
    /// render on a narrow terminal.
    pub summary: &'static str,
    pub content: &'static str,
}

pub const SKILLS: [Skill; 5] = [
    Skill {
        name: "pmk-feature-loop",
        summary: "the front door: route an idea into the right stage",
        content: include_str!("../.agents/skills/pmk-feature-loop/SKILL.md"),
    },
    Skill {
        name: "pmk-shape-idea",
        summary: "turn a rough idea into a spec you have agreed to",
        content: include_str!("../.agents/skills/pmk-shape-idea/SKILL.md"),
    },
    Skill {
        name: "pmk-build-safely",
        summary: "build a spec one small reviewed task at a time",
        content: include_str!("../.agents/skills/pmk-build-safely/SKILL.md"),
    },
    Skill {
        name: "pmk-verify-visually",
        summary: "drive a real browser before claiming it works",
        content: include_str!("../.agents/skills/pmk-verify-visually/SKILL.md"),
    },
    Skill {
        name: "pmk-jira-flow",
        summary: "keep the ticket's state matching reality",
        content: include_str!("../.agents/skills/pmk-jira-flow/SKILL.md"),
    },
];

pub fn skill_by_name(name: &str) -> Option<&'static Skill> {
    SKILLS.iter().find(|s| s.name == name)
}
