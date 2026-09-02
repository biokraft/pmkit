pub mod probes;
pub mod runner;

use comfy_table::{presets::UTF8_FULL, ContentArrangement, Table};
use probes::{Probe, ProbeStatus};

/// The doctor's report. Read-only: printing this never changes anything on the
/// machine.
pub fn table(probes: &[Probe]) -> String {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    // Wrap cell text to the terminal width instead of emitting one enormous
    // row that a normal 80/120-column terminal mangles. `Dynamic` only wraps
    // when comfy-table can detect a width (a real tty); when output is
    // piped -- as it is for most test harnesses, and for anyone redirecting
    // `pmkit doctor` to a file -- there is no width to detect, so fall back
    // to a fixed, conservative one.
    table.set_content_arrangement(ContentArrangement::Dynamic);
    if !table.is_tty() {
        table.set_width(100);
    }
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
            p.fix
                .as_ref()
                .map(|f| f.text().to_string())
                .unwrap_or_default(),
        ]);
    }
    table.to_string()
}
