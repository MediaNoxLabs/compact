//! Nonfatal checked recording diagnostics from original borrowed occurrences.
//! Source/call sites become small IDs synchronously; ranges expand after unwind.
use super::{
    binding::BoundOccurrences,
    diagnostic::{Capture, SourceDiagnostic},
    source_text::SourceSnapshots,
    transport::UntrustedTransport,
};
use crate::{
    RecordingGap,
    ir::StatefulCircuit,
    source_diagnostics::{RefusalObserver, RefusalSite},
};
use std::{cell::RefCell, path::Path};

#[derive(Debug)]
pub struct RecordingDiagnostic {
    pub circuit: String,
    /// Canonical typed refusal, including IR-node and resource-policy identity.
    /// Source ranges add context; they never reconstruct the cause from text.
    pub gap: RecordingGap,
    pub primary: Option<Box<SourceDiagnostic>>,
    /// Actual original direct call sites only. Pure DFS names are not sites.
    pub related_call_sites: Vec<SourceDiagnostic>,
}

#[derive(Default)]
struct Session {
    primary: Capture,
    related: Vec<Capture>,
}
pub(crate) struct Recorder<'context, 'ir> {
    bound: Option<&'context BoundOccurrences<'ir>>,
    transport: Option<&'context UntrustedTransport>,
    snapshots: Option<&'context SourceSnapshots>,
    cwd: &'context Path,
    session: RefCell<Session>,
    completed: RefCell<Vec<RecordingDiagnostic>>,
}
impl<'context, 'ir> Recorder<'context, 'ir> {
    pub(crate) fn new(
        bound: Option<&'context BoundOccurrences<'ir>>,
        transport: Option<&'context UntrustedTransport>,
        snapshots: Option<&'context SourceSnapshots>,
        cwd: &'context Path,
    ) -> Self {
        Self {
            bound,
            transport,
            snapshots,
            cwd,
            session: RefCell::new(Session::default()),
            completed: RefCell::new(Vec::new()),
        }
    }
    pub(crate) fn take_completed(&self) -> Vec<RecordingDiagnostic> {
        std::mem::take(&mut *self.completed.borrow_mut())
    }
}
impl RefusalObserver for Recorder<'_, '_> {
    fn refused(&self, site: RefusalSite<'_>) {
        self.session
            .borrow_mut()
            .primary
            .observe_site(site, self.bound, self.transport);
    }
    fn related(&self, site: RefusalSite<'_>) {
        let mut capture = Capture::default();
        capture.observe_site(site, self.bound, self.transport);
        self.session.borrow_mut().related.push(capture);
    }
    fn begin_recording_root(&self, _root: &StatefulCircuit) {
        *self.session.borrow_mut() = Session::default();
    }
    fn finish_recording_root(&self, root: &StatefulCircuit, gap: Option<&RecordingGap>) {
        let session = std::mem::take(&mut *self.session.borrow_mut());
        if root.internal {
            return;
        }
        let Some(gap) = gap else {
            return;
        };
        // Failed check and recursive certifier/lowerer frames have returned.
        let primary = session
            .primary
            .expand(self.transport, self.snapshots, self.cwd);
        let related_call_sites = session
            .related
            .into_iter()
            .filter_map(|capture| capture.expand(self.transport, self.snapshots, self.cwd))
            .map(|diagnostic| *diagnostic)
            .collect();
        self.completed.borrow_mut().push(RecordingDiagnostic {
            circuit: root.name.clone(),
            gap: gap.clone(),
            primary,
            related_call_sites,
        });
    }
}
