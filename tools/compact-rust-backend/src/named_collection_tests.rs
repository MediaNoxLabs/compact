// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Original collection failures; old owners are literal H197 routine bodies.
use crate::ir::{
    Constructor, LedgerField, LocalBinding, Parameter, ReturnPlan, StateReturn, TypeAlias,
    WitnessDeclaration,
};
use crate::source_diagnostics::Slot;
use crate::*;
use std::cell::RefCell;
mod guards;
mod old_reference;
mod transport;
mod workers;
type Event = (Option<(&'static str, usize)>, Slot);
#[derive(Default)]
struct Trace(RefCell<Vec<Event>>);
fn event(site: RefusalSite<'_>) -> Event {
    (
        site.owner.map(|owner| match owner {
            Owner::Type(value) => ("type", std::ptr::from_ref(value) as usize),
            Owner::StructField(value) => ("field", std::ptr::from_ref(value) as usize),
            Owner::Expr(value) => ("expr", std::ptr::from_ref(value) as usize),
            _ => panic!("not an original collector Type/member"),
        }),
        site.slot,
    )
}
impl RefusalObserver for Trace {
    fn refused(&self, site: RefusalSite<'_>) {
        self.0.borrow_mut().push(event(site));
    }
}
fn one(trace: &Trace, site: RefusalSite<'_>) {
    assert_eq!(trace.0.borrow().as_slice(), &[event(site)]);
}
fn product(name: &str, fields: Vec<StructField>) -> Type {
    Type::Struct {
        name: name.into(),
        fields,
    }
}
fn member(name: &str, ty: Type) -> StructField {
    StructField {
        name: name.into(),
        ty,
    }
}
fn enumeration(name: &str, variants: &[&str]) -> Type {
    Type::Enum {
        name: name.into(),
        variants: variants.iter().map(|s| (*s).into()).collect(),
    }
}
fn invalid() -> Type {
    product("crate", vec![])
}
fn default(ty: Type) -> Expr {
    Expr::Default { ty }
}
fn pure(body: Expr) -> PureCircuit {
    PureCircuit {
        source: None,
        name: "inspect".into(),
        internal: false,
        parameters: vec![],
        body,
        result: Type::Unit,
    }
}
fn stateful() -> StatefulCircuit {
    StatefulCircuit {
        source: None,
        name: "inspect".into(),
        internal: false,
        parameters: vec![],
        actions: vec![],
        result: Type::Unit,
        return_value: StateReturn::Unit,
    }
}
fn contract() -> Contract {
    Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        ledger_fields: vec![],
        circuits: vec![],
        stateful_circuits: vec![],
    }
}
fn same_public_error(source: &Contract, trace: &Trace) -> RenderError {
    let bare = render_with_capabilities(source)
        .err()
        .expect("originless failure");
    let observed = render_with_native_refusal(source, None, Some(trace))
        .err()
        .expect("same failure");
    assert_eq!(observed, bare);
    observed
}
type Collection = (
    Result<(), RenderError>,
    BTreeMap<String, Vec<StructField>>,
    BTreeMap<String, Vec<String>>,
    Vec<(String, usize)>,
);
fn collected<'ir>(
    operation: impl FnOnce(
        &mut BTreeMap<String, Vec<StructField>>,
        &mut BTreeMap<String, Vec<String>>,
        &mut type_declarations::DefinitionOrigins<'ir>,
    ) -> Result<(), RenderError>,
) -> Collection {
    let mut structs = BTreeMap::new();
    let mut enums = BTreeMap::new();
    let mut origins = type_declarations::DefinitionOrigins::new();
    let result = operation(&mut structs, &mut enums, &mut origins);
    let identity = origins
        .iter()
        .map(|(name, value)| (name.clone(), std::ptr::from_ref(*value) as usize))
        .collect();
    (result, structs, enums, identity)
}
