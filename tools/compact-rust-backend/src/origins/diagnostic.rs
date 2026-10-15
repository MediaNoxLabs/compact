//! Pure Expr occurrence capture is first-write-wins, even when unavailable.
//! Physical diagnostic data expands only after native recursion has unwound.
use super::{
    binding::BoundOccurrences,
    source_text::SourceSnapshots,
    status::SourceSnapshotFingerprint,
    transport::{
        UntrustedTransport,
        model::{Category, Extent, Label},
    },
};
use crate::{
    ir::Expr,
    source_diagnostics::{Owner, RefusalSite, Slot},
};
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourcePosition {
    pub line: u64,
    pub column: u64,
    /// Decoded-source character offset, not a raw UTF8 byte offset.
    pub offset: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceDiagnostic {
    pub address_id: usize,
    pub file_id: usize,
    pub descriptor_path: String,
    pub resolved_path: String,
    pub start: SourcePosition,
    pub end: SourcePosition,
    pub end_exclusive: bool,
    pub coordinate_system: &'static str,
    pub provenance: &'static str,
    pub category: &'static str,
    /// Explicit original occurrence slot; inherited ranges need not be token spans.
    pub selected_slot: &'static str,
    pub post_invocation_snapshot: Option<SourceSnapshotFingerprint>,
    /// Snapshot correspondence is not consumed-source authentication.
    pub range_matches_post_invocation_snapshot: bool,
}

#[derive(Default)]
pub(crate) enum Capture {
    #[default]
    NotCaptured,
    Captured(Option<(usize, Slot, bool)>),
}
impl Capture {
    pub(crate) fn observe(
        &mut self,
        source: &Expr,
        bound: Option<&BoundOccurrences<'_>>,
        transport: Option<&UntrustedTransport>,
    ) {
        self.observe_site(RefusalSite::node(Owner::Expr(source)), bound, transport);
    }
    pub(crate) fn observe_site(
        &mut self,
        site: RefusalSite<'_>,
        bound: Option<&BoundOccurrences<'_>>,
        transport: Option<&UntrustedTransport>,
    ) {
        if !matches!(self, Self::NotCaptured) {
            return;
        }
        let address = site.owner.and_then(|owner| {
            bound.and_then(|bound| match (owner, site.slot) {
                (Owner::Expr(expression), Slot::Node) => bound.expression_address(expression),
                _ => bound.occurrence_address(owner, site.slot),
            })
        });
        // Even an absent/unmapped origin must block enclosing parent fallback.
        let address = address.filter(|id| {
            transport.is_some_and(|transport| transport.rows()[*id].origin.0.is_some())
        });
        let expression = matches!(site.owner, Some(Owner::Expr(_)));
        *self = Self::Captured(address.map(|address| (address, site.slot, expression)));
    }

    pub(crate) fn expand(
        &self,
        transport: Option<&UntrustedTransport>,
        snapshots: Option<&SourceSnapshots>,
        cwd: &Path,
    ) -> Option<Box<SourceDiagnostic>> {
        let Self::Captured(Some((address_id, slot, expression))) = self else {
            return None;
        };
        let transport = transport?;
        let origin_id = transport.rows().get(*address_id)?.origin.0? as usize;
        let origin = transport.origins().get(origin_id)?;
        if !matches!(origin.extent, Extent::PhysicalSource {}) {
            return None;
        }
        let file_id = origin.file_id as usize;
        let file = transport.files().get(file_id)?;
        let position = |endpoint: &super::transport::model::Endpoint| SourcePosition {
            line: endpoint.line,
            column: endpoint.column,
            offset: endpoint.offset,
        };
        Some(Box::new(SourceDiagnostic {
            address_id: *address_id,
            file_id,
            descriptor_path: file.path.clone(),
            resolved_path: cwd.join(&file.path).to_string_lossy().into_owned(),
            start: position(&origin.start),
            end: position(&origin.end),
            end_exclusive: true,
            coordinate_system: "compact-decoded-source",
            provenance: match origin.label {
                Label::Provided => "provided",
                Label::Derived => "derived",
                Label::Inherited => "inherited",
            },
            category: match origin.category {
                Category::Program => "program",
                Category::Declaration => "declaration",
                Category::Type => "type",
                Category::Expression => "expression",
                Category::Argument => "argument",
                Category::CounterAmount => "counter_amount",
                Category::StateAction => "state_action",
                Category::ReturnPlan => "return_plan",
                Category::Identifier => "identifier",
                Category::Operation => "operation",
            },
            selected_slot: match slot {
                Slot::Node if *expression => "expr_node",
                Slot::Node => "node",
                Slot::Field(name) => name,
                Slot::Element { field, .. } => field,
            },
            post_invocation_snapshot: snapshots
                .and_then(|snapshots| snapshots.fingerprint(file_id)),
            range_matches_post_invocation_snapshot: snapshots
                .is_some_and(|snapshots| snapshots.range_matches_snapshot(origin)),
        }))
    }
}
