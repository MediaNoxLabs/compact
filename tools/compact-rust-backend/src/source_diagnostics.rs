//! Reference identity, never derived semantic equality or scalar data pointers.
use crate::ir::*;
use std::hash::{Hash, Hasher};

#[derive(Clone, Copy, Debug)]
pub(crate) enum Owner<'ir> {
    Contract(&'ir Contract),
    Alias(&'ir TypeAlias),
    Constructor(&'ir Constructor),
    ConstructorStep(&'ir ConstructorStep),
    Witness(&'ir WitnessDeclaration),
    Ledger(&'ir LedgerField),
    LedgerKind(&'ir LedgerFieldKind),
    Pure(&'ir PureCircuit),
    Stateful(&'ir StatefulCircuit),
    Parameter(&'ir Parameter),
    Binding(&'ir LocalBinding),
    Type(&'ir Type),
    StructField(&'ir StructField),
    Expr(&'ir Expr),
    Action(&'ir StateAction),
    StateReturn(&'ir StateReturn),
    ReturnPlan(&'ir ReturnPlan),
    CounterAmount(&'ir CounterAmount),
    SourcePoint(&'ir SourceLocation),
}

macro_rules! owner_identity {
    ($($variant:ident => $tag:literal),* $(,)?) => {
        impl Owner<'_> {
            fn identity(self) -> (u8, *const ()) {
                match self { $(Self::$variant(value) => ($tag, std::ptr::from_ref(value).cast()),)* }
            }
        }
    };
}
owner_identity! {
    Contract=>0, Alias=>1, Constructor=>2, ConstructorStep=>3, Witness=>4,
    Ledger=>5, LedgerKind=>6, Pure=>7, Stateful=>8, Parameter=>9, Binding=>10,
    Type=>11, StructField=>12, Expr=>13, Action=>14, StateReturn=>15,
    ReturnPlan=>16, CounterAmount=>17, SourcePoint=>18,
}
impl PartialEq for Owner<'_> {
    fn eq(&self, other: &Self) -> bool {
        let (tag, pointer) = self.identity();
        let (other_tag, other_pointer) = other.identity();
        tag == other_tag && std::ptr::eq(pointer, other_pointer)
    }
}
impl Eq for Owner<'_> {}
impl Hash for Owner<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let (tag, pointer) = self.identity();
        tag.hash(state);
        pointer.hash(state);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Slot {
    Node,
    Field(&'static str),
    Element { field: &'static str, index: usize },
}

/// An explicit original occurrence. None records unavailable without guessing.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RefusalSite<'source> {
    pub(crate) owner: Option<Owner<'source>>,
    pub(crate) slot: Slot,
}
impl<'source> RefusalSite<'source> {
    pub(crate) fn node(owner: Owner<'source>) -> Self {
        Self {
            owner: Some(owner),
            slot: Slot::Node,
        }
    }
    pub(crate) fn field(owner: Owner<'source>, field: &'static str) -> Self {
        Self {
            owner: Some(owner),
            slot: Slot::Field(field),
        }
    }
    pub(crate) fn unavailable() -> Self {
        Self {
            owner: None,
            slot: Slot::Node,
        }
    }
    pub(crate) fn element(owner: Owner<'source>, field: &'static str, index: usize) -> Self {
        Self {
            owner: Some(owner),
            slot: Slot::Element { field, index },
        }
    }
}
/// Caller-owned recording state; no global context, source checks or error replacement.
pub(crate) trait RefusalObserver {
    fn refused(&self, site: RefusalSite<'_>);
    fn related(&self, _site: RefusalSite<'_>) {}
    fn begin_recording_root(&self, _root: &StatefulCircuit) {}
    fn finish_recording_root(&self, _root: &StatefulCircuit, _gap: Option<&crate::RecordingGap>) {}
}
