// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Direct generic entry headers, selected frame priority, and silent probe controls.
use super::*;
use crate::ir::{Expr, LedgerFieldKind, Parameter};
use crate::source_diagnostics::Slot;
use std::cell::RefCell;

#[derive(Default)]
struct Trace(RefCell<Vec<(&'static str, usize, Slot)>>);
fn key(owner: Owner<'_>, slot: Slot) -> (&'static str, usize, Slot) {
    let (kind, pointer) = match owner {
        Owner::Stateful(value) => ("circuit", std::ptr::from_ref(value) as usize),
        Owner::Parameter(value) => ("parameter", std::ptr::from_ref(value) as usize),
        Owner::Type(value) => ("type", std::ptr::from_ref(value) as usize),
        Owner::Expr(value) => ("expr", std::ptr::from_ref(value) as usize),
        Owner::Action(value) => ("action", std::ptr::from_ref(value) as usize),
        Owner::Ledger(value) => ("ledger", std::ptr::from_ref(value) as usize),
        Owner::StateReturn(value) => ("return", std::ptr::from_ref(value) as usize),
        _ => panic!("unexpected stateful header control owner"),
    };
    (kind, pointer, slot)
}
impl RefusalObserver for Trace {
    fn refused(&self, site: RefusalSite<'_>) {
        self.0
            .borrow_mut()
            .push(key(site.owner.expect("original source"), site.slot));
    }
}
fn one(trace: &Trace, owner: Owner<'_>, slot: Slot) {
    assert_eq!(trace.0.borrow().as_slice(), &[key(owner, slot)]);
}
fn field(value: &str) -> Expr {
    Expr::FieldLiteral {
        value: value.into(),
    }
}
fn map_type() -> Type {
    Type::LedgerMap {
        key: Box::new(Type::Field),
        value: Box::new(Type::Field),
    }
}
fn source() -> StatefulCircuit {
    StatefulCircuit {
        source: None,
        name: "probe".into(),
        internal: false,
        parameters: vec![],
        actions: vec![],
        result: Type::Boolean,
        return_value: StateReturn::Expression {
            value: Expr::Boolean { value: true },
        },
    }
}
fn run(
    circuit: &StatefulCircuit,
    fields: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    callees: &HashMap<&str, &StatefulCircuit>,
    observer: Option<&dyn RefusalObserver>,
) -> Result<String, RenderError> {
    let pure = HashMap::new();
    let item = if observer.is_some() {
        render_stateful_circuit_observed(circuit, fields, witnesses, &pure, callees, observer)
    } else {
        render_stateful_circuit(circuit, fields, witnesses, &pure, callees)
    }?;
    Ok(quote::quote!(#item).to_string())
}
fn error(circuit: &StatefulCircuit, trace: &Trace, expected: RenderError) {
    let actual = run(
        circuit,
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        Some(trace),
    );
    assert_eq!(
        actual,
        run(
            circuit,
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            None
        )
    );
    assert_eq!(actual, Err(expected));
}

#[test]
fn generic_early_circuit_name_precedes_parameter_carrier_graph_and_body() {
    let mut circuit = source();
    circuit.name = "crate".into();
    circuit.parameters = vec![Parameter {
        name: "crate".into(),
        ty: map_type(),
    }];
    circuit.result = map_type();
    circuit.return_value = StateReturn::Expression {
        value: field("bad"),
    };
    let trace = Trace::default();
    error(
        &circuit,
        &trace,
        RenderError::InvalidIdentifier("crate".into()),
    );
    one(&trace, Owner::Stateful(&circuit), Slot::Field("name"));
}

#[test]
fn generic_parameter_name_precedes_same_carrier_and_later_result_body() {
    let mut circuit = source();
    circuit.parameters = vec![Parameter {
        name: "crate".into(),
        ty: map_type(),
    }];
    circuit.result = map_type();
    circuit.return_value = StateReturn::Expression {
        value: field("bad"),
    };
    let trace = Trace::default();
    error(
        &circuit,
        &trace,
        RenderError::InvalidIdentifier("crate".into()),
    );
    one(
        &trace,
        Owner::Parameter(&circuit.parameters[0]),
        Slot::Field("name"),
    );
}

#[test]
fn duplicate_insertion_precedes_later_carrier_but_earlier_carrier_precedes_duplicate() {
    let mut circuit = source();
    circuit.parameters = vec![
        Parameter {
            name: "same".into(),
            ty: Type::Field,
        },
        Parameter {
            name: "same".into(),
            ty: map_type(),
        },
    ];
    let trace = Trace::default();
    error(
        &circuit,
        &trace,
        RenderError::DuplicateParameter("same".into()),
    );
    one(
        &trace,
        Owner::Parameter(&circuit.parameters[1]),
        Slot::Field("name"),
    );
    circuit.parameters[0].ty = map_type();
    let trace = Trace::default();
    error(
        &circuit,
        &trace,
        RenderError::UnsupportedLedgerValueType(circuit.parameters[0].ty.clone()),
    );
    one(&trace, Owner::Type(&circuit.parameters[0].ty), Slot::Node);
}

#[test]
fn parameter_carrier_keeps_original_nested_leaf_before_later_bad_header() {
    let mut circuit = source();
    circuit.parameters = vec![
        Parameter {
            name: "valid".into(),
            ty: Type::Tuple {
                elements: vec![Type::Field, Type::Unsigned { max: "bad".into() }],
            },
        },
        Parameter {
            name: "crate".into(),
            ty: Type::Field,
        },
    ];
    let Type::Tuple { elements } = &circuit.parameters[0].ty else {
        panic!("fixture")
    };
    let trace = Trace::default();
    error(
        &circuit,
        &trace,
        RenderError::InvalidUnsignedMaximum("bad".into()),
    );
    one(&trace, Owner::Type(&elements[1]), Slot::Field("max"));
}

#[test]
fn original_result_carrier_refuses_before_body_without_replacing_child_when_supported() {
    let mut circuit = source();
    circuit.result = map_type();
    circuit.return_value = StateReturn::Expression {
        value: field("bad"),
    };
    let trace = Trace::default();
    error(
        &circuit,
        &trace,
        RenderError::UnsupportedLedgerValueType(circuit.result.clone()),
    );
    one(&trace, Owner::Type(&circuit.result), Slot::Node);
    circuit.result = Type::Boolean;
    let StateReturn::Expression { value } = &circuit.return_value else {
        panic!("fixture")
    };
    let trace = Trace::default();
    error(
        &circuit,
        &trace,
        RenderError::InvalidFieldLiteral("bad".into()),
    );
    one(&trace, Owner::Expr(value), Slot::Node);
}

#[test]
fn graph_priority_depends_on_original_frame_eligibility_and_header_positions() {
    let mut looped = source();
    looped.name = "loop".into();
    looped.result = Type::Unit;
    looped.return_value = StateReturn::Unit;
    looped.actions = vec![StateAction::CircuitCall {
        name: "loop".into(),
        arguments: vec![],
    }];
    let callees = HashMap::from([("loop", &looped)]);
    let closing = &looped.actions[0];
    let mut circuit = source();
    circuit.name = "crate".into();
    circuit.result = Type::Unit;
    circuit.return_value = StateReturn::Unit;
    circuit.actions = vec![StateAction::CircuitCall {
        name: "loop".into(),
        arguments: vec![],
    }];
    circuit.parameters = vec![Parameter {
        name: "crate".into(),
        ty: Type::Field,
    }];
    let trace = Trace::default();
    let actual = run(
        &circuit,
        &HashMap::new(),
        &HashMap::new(),
        &callees,
        Some(&trace),
    );
    assert_eq!(
        actual,
        run(&circuit, &HashMap::new(), &HashMap::new(), &callees, None)
    );
    assert_eq!(
        actual,
        Err(RenderError::UnsupportedStatefulCall("loop".into()))
    );
    one(&trace, Owner::Action(closing), Slot::Field("name"));
    // Non-Unit bypasses the frame graph probe; generic parameter validation now
    // remains before its own graph scan, while result carrier is after graphs.
    circuit.name = "probe".into();
    circuit.result = map_type();
    circuit.return_value = StateReturn::Expression {
        value: Expr::Boolean { value: true },
    };
    let trace = Trace::default();
    let actual = run(
        &circuit,
        &HashMap::new(),
        &HashMap::new(),
        &callees,
        Some(&trace),
    );
    assert_eq!(
        actual,
        run(&circuit, &HashMap::new(), &HashMap::new(), &callees, None)
    );
    assert_eq!(actual, Err(RenderError::InvalidIdentifier("crate".into())));
    one(
        &trace,
        Owner::Parameter(&circuit.parameters[0]),
        Slot::Field("name"),
    );
    circuit.parameters.clear();
    let trace = Trace::default();
    let actual = run(
        &circuit,
        &HashMap::new(),
        &HashMap::new(),
        &callees,
        Some(&trace),
    );
    assert_eq!(
        actual,
        run(&circuit, &HashMap::new(), &HashMap::new(), &callees, None)
    );
    assert_eq!(
        actual,
        Err(RenderError::UnsupportedStatefulCall("loop".into()))
    );
    one(&trace, Owner::Action(closing), Slot::Field("name"));
}

#[test]
fn selected_frame_child_still_precedes_its_late_circuit_name() {
    let target = LedgerField {
        source: None,
        id: "stored".into(),
        index: 0,
        path: vec![0],
        declaration: LedgerFieldKind::Cell { ty: Type::Field },
    };
    let witness = WitnessDeclaration {
        source: None,
        name: "input".into(),
        parameters: vec![Parameter {
            name: "value".into(),
            ty: Type::Field,
        }],
        result: Type::Field,
    };
    let fields = HashMap::from([("stored", &target)]);
    let witnesses = HashMap::from([("input", &witness)]);
    let mut circuit = source();
    circuit.name = "crate".into();
    circuit.result = Type::Unit;
    circuit.return_value = StateReturn::Unit;
    circuit.actions = vec![StateAction::CellWrite {
        field: "stored".into(),
        index: 0,
        value: Expr::WitnessCall {
            name: "input".into(),
            arguments: vec![field("bad")],
        },
    }];
    let StateAction::CellWrite {
        value: Expr::WitnessCall { arguments, .. },
        ..
    } = &circuit.actions[0]
    else {
        panic!("fixture")
    };
    let trace = Trace::default();
    let actual = run(&circuit, &fields, &witnesses, &HashMap::new(), Some(&trace));
    assert_eq!(
        actual,
        run(&circuit, &fields, &witnesses, &HashMap::new(), None)
    );
    assert_eq!(actual, Err(RenderError::InvalidFieldLiteral("bad".into())));
    one(&trace, Owner::Expr(&arguments[0]), Slot::Node);
    let StateAction::CellWrite {
        value: Expr::WitnessCall { arguments, .. },
        ..
    } = &mut circuit.actions[0]
    else {
        panic!("fixture")
    };
    arguments[0] = field("1");
    let trace = Trace::default();
    let actual = run(&circuit, &fields, &witnesses, &HashMap::new(), Some(&trace));
    assert_eq!(
        actual,
        run(&circuit, &fields, &witnesses, &HashMap::new(), None)
    );
    assert_eq!(actual, Err(RenderError::InvalidIdentifier("crate".into())));
    one(&trace, Owner::Stateful(&circuit), Slot::Field("name"));
}

#[test]
fn swallowed_simple_leaf_name_probes_do_not_emit_extra_refusals() {
    let mut target = LedgerField {
        source: None,
        id: "stored".into(),
        index: 0,
        path: vec![0],
        declaration: LedgerFieldKind::Cell { ty: Type::Boolean },
    };
    let mut circuit = source();
    circuit.name = "crate".into();
    circuit.result = Type::Unit;
    circuit.return_value = StateReturn::Unit;
    circuit.actions = vec![StateAction::CellWrite {
        field: "stored".into(),
        index: 0,
        value: Expr::Boolean { value: true },
    }];
    let trace = Trace::default();
    let actual = run(
        &circuit,
        &HashMap::from([("stored", &target)]),
        &HashMap::new(),
        &HashMap::new(),
        Some(&trace),
    );
    assert_eq!(
        actual,
        run(
            &circuit,
            &HashMap::from([("stored", &target)]),
            &HashMap::new(),
            &HashMap::new(),
            None
        )
    );
    assert_eq!(actual, Err(RenderError::InvalidIdentifier("crate".into())));
    one(&trace, Owner::Stateful(&circuit), Slot::Field("name"));
    circuit.name = "probe".into();
    target.id = "crate".into();
    let StateAction::CellWrite { field, .. } = &mut circuit.actions[0] else {
        panic!("fixture")
    };
    *field = "crate".into();
    let trace = Trace::default();
    let actual = run(
        &circuit,
        &HashMap::from([("crate", &target)]),
        &HashMap::new(),
        &HashMap::new(),
        Some(&trace),
    );
    assert_eq!(
        actual,
        run(
            &circuit,
            &HashMap::from([("crate", &target)]),
            &HashMap::new(),
            &HashMap::new(),
            None
        )
    );
    assert_eq!(actual, Err(RenderError::InvalidIdentifier("crate".into())));
    one(&trace, Owner::Ledger(&target), Slot::Field("id"));
}

#[test]
fn valid_generic_headers_keep_raw_lookup_generated_positions_and_complete_output() {
    let mut circuit = source();
    circuit.parameters = vec![
        Parameter {
            name: "value$0".into(),
            ty: Type::Field,
        },
        Parameter {
            name: "r#value".into(),
            ty: Type::Boolean,
        },
    ];
    circuit.actions = vec![StateAction::Expression {
        value: Expr::Parameter {
            name: "value$0".into(),
        },
    }];
    circuit.return_value = StateReturn::Expression {
        value: Expr::Parameter {
            name: "r#value".into(),
        },
    };
    for internal in [false, true] {
        circuit.internal = internal;
        let trace = Trace::default();
        let observed = run(
            &circuit,
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            Some(&trace),
        )
        .unwrap();
        assert_eq!(
            observed,
            run(
                &circuit,
                &HashMap::new(),
                &HashMap::new(),
                &HashMap::new(),
                None
            )
            .unwrap()
        );
        assert!(observed.contains("__compact_param_0") && observed.contains("__compact_param_1"));
        assert!(trace.0.borrow().is_empty());
    }
}

#[test]
fn equal_live_parameter_and_circuit_names_keep_distinct_original_identity() {
    let mut first = source();
    first.parameters = vec![Parameter {
        name: "crate".into(),
        ty: Type::Field,
    }];
    let circuits = [first.clone(), first];
    let mut ids = Vec::new();
    for circuit in &circuits {
        let trace = Trace::default();
        error(
            circuit,
            &trace,
            RenderError::InvalidIdentifier("crate".into()),
        );
        one(
            &trace,
            Owner::Parameter(&circuit.parameters[0]),
            Slot::Field("name"),
        );
        ids.push(key(
            Owner::Parameter(&circuit.parameters[0]),
            Slot::Field("name"),
        ));
    }
    assert_ne!(ids[0], ids[1]);
    let mut first = source();
    first.name = "crate".into();
    let circuits = [first.clone(), first];
    let mut ids = Vec::new();
    for circuit in &circuits {
        let trace = Trace::default();
        error(
            circuit,
            &trace,
            RenderError::InvalidIdentifier("crate".into()),
        );
        one(&trace, Owner::Stateful(circuit), Slot::Field("name"));
        ids.push(key(Owner::Stateful(circuit), Slot::Field("name")));
    }
    assert_ne!(ids[0], ids[1]);
}
