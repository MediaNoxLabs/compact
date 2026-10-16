//! Add issued ranges after the unchanged compiler cause; never infer a site.
use compact_rust_backend::{
    NativeCompileFailure, NativeFailureCause, RecordingDiagnostic, SourceDiagnostic,
};
use std::error::Error;

/// CLI-only failure: the old primary message plus notes prepared after unwind.
/// No borrowed source, observer, transport or semantic authority escapes here.
#[derive(Debug)]
pub(crate) struct RecordingGateFailure {
    pub(crate) message: String,
    pub(crate) notes: Vec<String>,
}
impl std::fmt::Display for RecordingGateFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}
impl Error for RecordingGateFailure {}

pub(crate) fn failure(compiler: &str, error: &(dyn Error + 'static)) {
    if let Some(failure) = error.downcast_ref::<RecordingGateFailure>() {
        for note in &failure.notes {
            eprintln!("{compiler}: {note}");
        }
        return;
    }
    let Some(failure) = error.downcast_ref::<NativeCompileFailure>() else {
        return;
    };
    if let Some(source) = failure.diagnostic.as_deref() {
        eprintln!("{compiler}: {}", range_note(source));
    }
    if matches!(&failure.cause, NativeFailureCause::RequireRecording(_)) {
        for note in recording_notes(&failure.recording_diagnostics) {
            eprintln!("{compiler}: {note}");
        }
    }
}

pub(crate) fn recording_notes(diagnostics: &[RecordingDiagnostic]) -> Vec<String> {
    let mut notes = Vec::new();
    for diagnostic in diagnostics {
        if let Some(source) = diagnostic.primary.as_deref() {
            notes.push(format!(
                "recording gap for {:?}: {}",
                diagnostic.circuit,
                range_note(source)
            ));
        }
        for source in &diagnostic.related_call_sites {
            notes.push(format!(
                "related call for {:?}: {}",
                diagnostic.circuit,
                range_note(source)
            ));
        }
    }
    notes
}

fn range_note(source: &SourceDiagnostic) -> String {
    let correspondence = if source.range_matches_post_invocation_snapshot {
        "range matches post-invocation snapshot"
    } else {
        "post-invocation snapshot correspondence unverified"
    };
    format!(
        "source range {:?} line {} char {} to line {} char {} (end exclusive; {}; {}/{}; slot {}; {correspondence})",
        source.resolved_path,
        source.start.line,
        source.start.column,
        source.end.line,
        source.end.column,
        source.coordinate_system,
        source.provenance,
        source.category,
        source.selected_slot,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use compact_rust_backend::SourcePosition;

    #[test]
    fn inherited_range_is_honest_about_extent_and_post_invocation_snapshot() {
        let source = SourceDiagnostic {
            address_id: 1,
            file_id: 0,
            descriptor_path: "input.compact".into(),
            resolved_path: "/example/input.compact".into(),
            start: SourcePosition {
                line: 3,
                column: 2,
                offset: 12,
            },
            end: SourcePosition {
                line: 5,
                column: 1,
                offset: 40,
            },
            end_exclusive: true,
            coordinate_system: "compact-decoded-source",
            provenance: "inherited",
            category: "expression",
            selected_slot: "message",
            post_invocation_snapshot: None,
            range_matches_post_invocation_snapshot: false,
        };
        let text = range_note(&source);
        assert!(text.contains("line 3 char 2 to line 5 char 1"));
        assert!(
            text.contains(
                "end exclusive; compact-decoded-source; inherited/expression; slot message"
            )
        );
        assert!(text.contains("post-invocation snapshot correspondence unverified"));
        assert!(!text.contains("authenticated"));
    }
}
