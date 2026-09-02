pub mod probes;
pub mod runner;

use comfy_table::{presets::UTF8_FULL, Table};
use probes::{Probe, ProbeStatus};

/// The doctor's report. Read-only: printing this never changes anything on the
/// machine.
pub fn table(probes: &[Probe]) -> String {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(vec!["TOOL", "STATE", "WHY IT MATTERS", "FIX"]);
    for p in probes {
        let (state, detail) = match &p.status {
            ProbeStatus::Ok(d) => ("ok", d.clone()),
            ProbeStatus::Missing => ("missing", String::new()),
            ProbeStatus::Broken(d) => ("needs attention", d.clone()),
        };
        table.add_row(vec![
            p.name.to_string(),
            if detail.is_empty() {
                state.to_string()
            } else {
                format!("{state} — {detail}")
            },
            p.why.to_string(),
            p.fix.clone().unwrap_or_default(),
        ]);
    }
    table.to_string()
}
