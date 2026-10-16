//! Local borrowed observer: only small selected identities are mutable.
use super::{
    binding::BoundOccurrences,
    diagnostic::{Capture, SourceDiagnostic},
    source_text::SourceSnapshots,
    transport::UntrustedTransport,
};
use crate::source_diagnostics::{Owner, RefusalObserver, RefusalSite, Slot};
use std::{cell::RefCell, path::Path};
pub(super) struct NativeObserver<'context, 'ir> {
    bound: Option<&'context BoundOccurrences<'ir>>,
    transport: Option<&'context UntrustedTransport>,
    capture: RefCell<Capture>,
}
impl<'context, 'ir> NativeObserver<'context, 'ir> {
    pub(super) fn new(
        bound: Option<&'context BoundOccurrences<'ir>>,
        transport: Option<&'context UntrustedTransport>,
    ) -> Self {
        Self {
            bound,
            transport,
            capture: RefCell::new(Capture::default()),
        }
    }
    pub(super) fn expand(
        &self,
        snapshots: Option<&SourceSnapshots>,
        cwd: &Path,
    ) -> Option<Box<SourceDiagnostic>> {
        self.capture.borrow().expand(self.transport, snapshots, cwd)
    }
}
impl RefusalObserver for NativeObserver<'_, '_> {
    fn refused(&self, site: RefusalSite<'_>) {
        let mut capture = self.capture.borrow_mut();
        if let (Some(Owner::Expr(expression)), Slot::Node) = (site.owner, site.slot) {
            capture.observe(expression, self.bound, self.transport);
        } else {
            capture.observe_site(site, self.bound, self.transport);
        }
    }
}
