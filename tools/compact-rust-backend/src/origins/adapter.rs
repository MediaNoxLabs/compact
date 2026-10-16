//! Production composition uses validated rows, never the example JSON adapter.
use super::{
    binding::addresses::{Address, AddressAccess, Edge},
    transport,
};

impl AddressAccess for transport::UntrustedTransport {
    fn len(&self) -> usize {
        self.rows().len()
    }
    fn address(&self, id: usize) -> Address<'_> {
        let row = &self.rows()[id];
        // Codec validation bounds every ID/index by the 131072-row table and
        // checks dense indices before this adapter is reachable (32/64-bit).
        Address {
            parent: row.parent.0.map(|id| id as usize),
            edge: match &row.edge {
                transport::model::Edge::Root(_) => Edge::Root,
                transport::model::Edge::Field(edge) => Edge::Field(&edge.field),
                transport::model::Edge::Index(edge) => Edge::Index(edge.index as usize),
            },
            kind: row.kind.0.as_deref(),
        }
    }
}
