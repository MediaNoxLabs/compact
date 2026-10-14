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

//! Exact existing return guard order and original source participants.
use super::*;
use crate::ir::{Expr, ReturnPlan};
use crate::source_diagnostics::Slot;
use std::cell::RefCell;

#[derive(Default)]
struct Trace(RefCell<Vec<(&'static str, usize, Slot)>>);
fn key(owner: Owner<'_>, slot: Slot) -> (&'static str, usize, Slot) {
    let (kind, pointer) = match owner {
        Owner::StateReturn(value) => ("return", std::ptr::from_ref(value) as usize),
        Owner::Type(value) => ("type", std::ptr::from_ref(value) as usize),
        Owner::Expr(value) => ("expr", std::ptr::from_ref(value) as usize),
        Owner::Ledger(value) => ("ledger", std::ptr::from_ref(value) as usize),
        _ => panic!("unexpected native return source"),
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
fn literal(value: &str) -> Expr {
    Expr::FieldLiteral {
        value: value.into(),
    }
}
fn parameter(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}
fn uint64() -> Type {
    Type::Unsigned {
        max: u64::MAX.to_string(),
    }
}
fn digest() -> Type {
    Type::Struct {
        name: "MerkleTreeDigest".into(),
        fields: vec![StructField {
            name: "field".into(),
            ty: Type::Field,
        }],
    }
}
fn maybe() -> Type {
    Type::Struct {
        name: "Maybe".into(),
        fields: vec![
            StructField {
                name: "is_some".into(),
                ty: Type::Boolean,
            },
            StructField {
                name: "value".into(),
                ty: Type::Field,
            },
        ],
    }
}
fn ledger(id: &str, declaration: LedgerFieldKind) -> LedgerField {
    LedgerField {
        source: None,
        id: id.into(),
        index: 0,
        path: vec![0],
        declaration,
    }
}
fn source(result: Type, return_value: StateReturn) -> StatefulCircuit {
    StatefulCircuit {
        source: None,
        name: "query".into(),
        internal: false,
        parameters: vec![],
        actions: vec![],
        result,
        return_value,
    }
}
fn cases(id: &str) -> Vec<(LedgerField, StatefulCircuit)> {
    let cell = || ledger(id, LedgerFieldKind::Cell { ty: Type::Field });
    let counter = || ledger(id, LedgerFieldKind::Counter);
    let list = || ledger(id, LedgerFieldKind::List { ty: Type::Field });
    let set = || ledger(id, LedgerFieldKind::Set { ty: Type::Field });
    let map = || {
        ledger(
            id,
            LedgerFieldKind::Map {
                key: Type::Field,
                value: Type::Field,
            },
        )
    };
    let plain = || {
        ledger(
            id,
            LedgerFieldKind::MerkleTree {
                depth: 8,
                ty: Type::Field,
            },
        )
    };
    let historic = || {
        ledger(
            id,
            LedgerFieldKind::HistoricMerkleTree {
                depth: 8,
                ty: Type::Field,
            },
        )
    };
    vec![
        (
            cell(),
            source(
                Type::Field,
                StateReturn::CellRead {
                    field: id.into(),
                    index: 0,
                },
            ),
        ),
        (
            counter(),
            source(
                uint64(),
                StateReturn::CounterRead {
                    field: id.into(),
                    index: 0,
                },
            ),
        ),
        (
            list(),
            source(
                uint64(),
                StateReturn::ListLength {
                    field: id.into(),
                    index: 0,
                },
            ),
        ),
        (
            list(),
            source(
                Type::Boolean,
                StateReturn::ListIsEmpty {
                    field: id.into(),
                    index: 0,
                },
            ),
        ),
        (
            list(),
            source(
                maybe(),
                StateReturn::ListHead {
                    field: id.into(),
                    index: 0,
                },
            ),
        ),
        (
            set(),
            source(
                Type::Boolean,
                StateReturn::SetMember {
                    field: id.into(),
                    index: 0,
                    value: parameter("item"),
                },
            ),
        ),
        (
            set(),
            source(
                uint64(),
                StateReturn::SetSize {
                    field: id.into(),
                    index: 0,
                },
            ),
        ),
        (
            set(),
            source(
                Type::Boolean,
                StateReturn::SetIsEmpty {
                    field: id.into(),
                    index: 0,
                },
            ),
        ),
        (
            map(),
            source(
                Type::Boolean,
                StateReturn::MapMember {
                    field: id.into(),
                    index: 0,
                    key: parameter("key"),
                },
            ),
        ),
        (
            map(),
            source(
                Type::Field,
                StateReturn::MapLookup {
                    field: id.into(),
                    index: 0,
                    key: parameter("key"),
                },
            ),
        ),
        (
            map(),
            source(
                uint64(),
                StateReturn::MapSize {
                    field: id.into(),
                    index: 0,
                },
            ),
        ),
        (
            map(),
            source(
                Type::Boolean,
                StateReturn::MapIsEmpty {
                    field: id.into(),
                    index: 0,
                },
            ),
        ),
        (
            plain(),
            source(
                Type::Boolean,
                StateReturn::MerkleIsFull {
                    field: id.into(),
                    index: 0,
                },
            ),
        ),
        (
            historic(),
            source(
                Type::Boolean,
                StateReturn::HistoricMerkleIsFull {
                    field: id.into(),
                    index: 0,
                },
            ),
        ),
        (
            plain(),
            source(
                Type::Boolean,
                StateReturn::MerkleCheckRoot {
                    field: id.into(),
                    index: 0,
                    root: parameter("root"),
                },
            ),
        ),
        (
            historic(),
            source(
                Type::Boolean,
                StateReturn::HistoricMerkleCheckRoot {
                    field: id.into(),
                    index: 0,
                    root: parameter("root"),
                },
            ),
        ),
    ]
}
fn run(
    circuit: &StatefulCircuit,
    fields: &HashMap<&str, &LedgerField>,
    observer: Option<&dyn RefusalObserver>,
) -> (Result<String, RenderError>, String, usize, bool, usize) {
    let witnesses = HashMap::new();
    let circuits = HashMap::new();
    let stateful_circuits = HashMap::new();
    let field_type = Type::Field;
    let root_type = digest();
    let parameters = HashMap::from([
        ("item", (&field_type, syn::parse_quote!(item))),
        ("key", (&field_type, syn::parse_quote!(key))),
        ("root", (&root_type, syn::parse_quote!(root))),
    ]);
    let environment = NativeExpressionContext {
        witnesses: &witnesses,
        circuits: &circuits,
        stateful_circuits: &stateful_circuits,
        ledger_fields: fields,
        observer,
    };
    let result_ty = crate::rust_type(&circuit.result).expect("supported fixture result annotation");
    let mut statements = vec![syn::parse_quote!(let __guard_seed = ();)];
    let mut next_temp = 17;
    let mut uses_witness = false;
    let result = if observer.is_some() {
        lower_observed(
            circuit,
            &parameters,
            &parameters,
            &environment,
            &result_ty,
            ReturnOutput {
                statements: &mut statements,
                next_temp: &mut next_temp,
                uses_witness: &mut uses_witness,
            },
        )
    } else {
        lower(
            circuit,
            &parameters,
            &parameters,
            &witnesses,
            &circuits,
            &stateful_circuits,
            fields,
            &result_ty,
            &mut statements,
            &mut next_temp,
            &mut uses_witness,
        )
    }
    .map(|value| quote::quote!(#value).to_string());
    let length = statements.len();
    (
        result,
        quote::quote!(#(#statements)*).to_string(),
        next_temp,
        uses_witness,
        length,
    )
}
fn error(
    circuit: &StatefulCircuit,
    fields: &HashMap<&str, &LedgerField>,
    trace: &Trace,
    expected: RenderError,
    next: usize,
) {
    let observed = run(circuit, fields, Some(trace));
    assert_eq!(
        observed,
        run(circuit, fields, None),
        "exact error/output/temp/witness state"
    );
    assert_eq!(observed.0, Err(expected));
    assert_eq!((observed.2, observed.3, observed.4), (next, false, 1));
    assert_eq!(
        observed.1,
        quote::quote!(let __guard_seed = ();).to_string()
    );
}

#[test]
fn all_sixteen_ledger_returns_lookup_before_target_result_or_operand_checks() {
    for (_, circuit) in cases("absent") {
        let trace = Trace::default();
        error(
            &circuit,
            &HashMap::new(),
            &trace,
            RenderError::UnknownLedgerField("absent".into()),
            17,
        );
        one(
            &trace,
            Owner::StateReturn(&circuit.return_value),
            Slot::Field("field"),
        );
    }
}

#[test]
fn distinct_kind_then_index_guards_keep_original_return_field_or_index() {
    for (mut target, mut circuit) in cases("crate") {
        let distinct = matches!(
            circuit.return_value,
            StateReturn::CellRead { .. }
                | StateReturn::ListHead { .. }
                | StateReturn::SetMember { .. }
                | StateReturn::MapMember { .. }
                | StateReturn::MapLookup { .. }
        );
        let tree = matches!(
            circuit.return_value,
            StateReturn::MerkleIsFull { .. }
                | StateReturn::HistoricMerkleIsFull { .. }
                | StateReturn::MerkleCheckRoot { .. }
                | StateReturn::HistoricMerkleCheckRoot { .. }
        );
        if !distinct && !tree {
            continue;
        }
        circuit.result = Type::Unit;
        let wrong = ledger("crate", LedgerFieldKind::Counter);
        let trace = Trace::default();
        error(
            &circuit,
            &HashMap::from([("crate", &wrong)]),
            &trace,
            RenderError::UnknownLedgerField("crate".into()),
            17,
        );
        one(
            &trace,
            Owner::StateReturn(&circuit.return_value),
            Slot::Field("field"),
        );
        target.index = 1;
        let trace = Trace::default();
        error(
            &circuit,
            &HashMap::from([("crate", &target)]),
            &trace,
            RenderError::UnknownLedgerField("crate".into()),
            17,
        );
        one(
            &trace,
            Owner::StateReturn(&circuit.return_value),
            if distinct {
                Slot::Field("index")
            } else {
                Slot::Node
            },
        );
    }
}

#[test]
fn combined_kind_index_and_historic_returns_keep_whole_original_node() {
    for (mut target, circuit) in cases("crate") {
        let combined_kind = matches!(
            circuit.return_value,
            StateReturn::CounterRead { .. }
                | StateReturn::ListLength { .. }
                | StateReturn::ListIsEmpty { .. }
                | StateReturn::SetSize { .. }
                | StateReturn::SetIsEmpty { .. }
                | StateReturn::MapSize { .. }
                | StateReturn::MapIsEmpty { .. }
        );
        let tree = matches!(
            circuit.return_value,
            StateReturn::MerkleIsFull { .. }
                | StateReturn::HistoricMerkleIsFull { .. }
                | StateReturn::MerkleCheckRoot { .. }
                | StateReturn::HistoricMerkleCheckRoot { .. }
        );
        if !combined_kind && !tree {
            continue;
        }
        let wrong_kind = if matches!(circuit.return_value, StateReturn::CounterRead { .. }) {
            LedgerFieldKind::Cell { ty: Type::Field }
        } else if tree {
            match &target.declaration {
                LedgerFieldKind::HistoricMerkleTree { .. } => LedgerFieldKind::MerkleTree {
                    depth: 8,
                    ty: Type::Field,
                },
                LedgerFieldKind::MerkleTree { .. } => LedgerFieldKind::HistoricMerkleTree {
                    depth: 8,
                    ty: Type::Field,
                },
                _ => panic!("fixture"),
            }
        } else {
            LedgerFieldKind::Counter
        };
        let wrong = ledger("crate", wrong_kind);
        let trace = Trace::default();
        error(
            &circuit,
            &HashMap::from([("crate", &wrong)]),
            &trace,
            RenderError::UnknownLedgerField("crate".into()),
            17,
        );
        one(
            &trace,
            Owner::StateReturn(&circuit.return_value),
            Slot::Node,
        );
        target.index = 1;
        let trace = Trace::default();
        error(
            &circuit,
            &HashMap::from([("crate", &target)]),
            &trace,
            RenderError::UnknownLedgerField("crate".into()),
            17,
        );
        one(
            &trace,
            Owner::StateReturn(&circuit.return_value),
            Slot::Node,
        );
    }
}

#[test]
fn every_new_declared_result_relation_uses_original_annotation_before_identifier() {
    let unit = source(Type::Field, StateReturn::Unit);
    let trace = Trace::default();
    error(
        &unit,
        &HashMap::new(),
        &trace,
        RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Unit,
        },
        17,
    );
    one(&trace, Owner::Type(&unit.result), Slot::Node);
    for (target, mut circuit) in cases("crate") {
        let expected = if matches!(circuit.return_value, StateReturn::CellRead { .. }) {
            Type::Unit
        } else {
            circuit.result.clone()
        };
        let actual = if matches!(circuit.return_value, StateReturn::CellRead { .. }) {
            Type::Field
        } else {
            Type::Unit
        };
        circuit.result = Type::Unit;
        let trace = Trace::default();
        error(
            &circuit,
            &HashMap::from([("crate", &target)]),
            &trace,
            RenderError::TypeMismatch { expected, actual },
            17,
        );
        one(&trace, Owner::Type(&circuit.result), Slot::Node);
    }
}

#[test]
fn set_and_check_root_result_precede_operand_but_map_key_precedes_result() {
    for (target, mut circuit) in cases("crate") {
        let operand = match &mut circuit.return_value {
            StateReturn::SetMember { value, .. } => value,
            StateReturn::MerkleCheckRoot { root, .. }
            | StateReturn::HistoricMerkleCheckRoot { root, .. } => root,
            StateReturn::MapMember { key, .. } | StateReturn::MapLookup { key, .. } => key,
            _ => continue,
        };
        *operand = parameter("later");
        circuit.result = Type::Unit;
        let map = matches!(
            circuit.return_value,
            StateReturn::MapMember { .. } | StateReturn::MapLookup { .. }
        );
        let trace = Trace::default();
        if map {
            error(
                &circuit,
                &HashMap::from([("crate", &target)]),
                &trace,
                RenderError::UnknownParameter("later".into()),
                17,
            );
            let (StateReturn::MapMember { key, .. } | StateReturn::MapLookup { key, .. }) =
                &circuit.return_value
            else {
                panic!("fixture")
            };
            one(&trace, Owner::Expr(key), Slot::Node);
        } else {
            error(
                &circuit,
                &HashMap::from([("crate", &target)]),
                &trace,
                RenderError::TypeMismatch {
                    expected: Type::Boolean,
                    actual: Type::Unit,
                },
                17,
            );
            one(&trace, Owner::Type(&circuit.result), Slot::Node);
        }
    }
    let target = ledger(
        "crate",
        LedgerFieldKind::Map {
            key: Type::Field,
            value: Type::Field,
        },
    );
    for value in [
        StateReturn::MapMember {
            field: "crate".into(),
            index: 0,
            key: Expr::Boolean { value: true },
        },
        StateReturn::MapLookup {
            field: "crate".into(),
            index: 0,
            key: Expr::Boolean { value: true },
        },
    ] {
        let circuit = source(Type::Unit, value);
        let (StateReturn::MapMember { key, .. } | StateReturn::MapLookup { key, .. }) =
            &circuit.return_value
        else {
            panic!("fixture")
        };
        let trace = Trace::default();
        error(
            &circuit,
            &HashMap::from([("crate", &target)]),
            &trace,
            RenderError::TypeMismatch {
                expected: Type::Field,
                actual: Type::Boolean,
            },
            17,
        );
        one(&trace, Owner::Expr(key), Slot::Node);
    }
}

#[test]
fn each_outer_root_value_and_key_type_guard_retains_original_operand() {
    for (target, mut circuit) in cases("crate") {
        let expected = match &mut circuit.return_value {
            StateReturn::SetMember { value, .. } => {
                *value = Expr::Boolean { value: true };
                Type::Field
            }
            StateReturn::MerkleCheckRoot { root, .. }
            | StateReturn::HistoricMerkleCheckRoot { root, .. } => {
                *root = Expr::Boolean { value: true };
                digest()
            }
            StateReturn::MapMember { key, .. } | StateReturn::MapLookup { key, .. } => {
                *key = Expr::Boolean { value: true };
                Type::Field
            }
            _ => continue,
        };
        let original = match &circuit.return_value {
            StateReturn::SetMember { value, .. } => value,
            StateReturn::MerkleCheckRoot { root, .. }
            | StateReturn::HistoricMerkleCheckRoot { root, .. } => root,
            StateReturn::MapMember { key, .. } | StateReturn::MapLookup { key, .. } => key,
            _ => panic!("fixture"),
        };
        let trace = Trace::default();
        error(
            &circuit,
            &HashMap::from([("crate", &target)]),
            &trace,
            RenderError::TypeMismatch {
                expected,
                actual: Type::Boolean,
            },
            17,
        );
        one(&trace, Owner::Expr(original), Slot::Node);
    }
}

#[test]
fn first_actual_child_refusal_is_not_replaced_by_new_outer_guard_or_ledger_name() {
    for (target, mut circuit) in cases("crate") {
        let operand = match &mut circuit.return_value {
            StateReturn::SetMember { value, .. } => value,
            StateReturn::MerkleCheckRoot { root, .. }
            | StateReturn::HistoricMerkleCheckRoot { root, .. } => root,
            StateReturn::MapMember { key, .. } | StateReturn::MapLookup { key, .. } => key,
            _ => continue,
        };
        *operand = Expr::Add {
            left: Box::new(literal("bad")),
            right: Box::new(parameter("later")),
        };
        if matches!(
            circuit.return_value,
            StateReturn::MapMember { .. } | StateReturn::MapLookup { .. }
        ) {
            circuit.result = Type::Unit;
        }
        let original = match &circuit.return_value {
            StateReturn::SetMember { value, .. } => value,
            StateReturn::MerkleCheckRoot { root, .. }
            | StateReturn::HistoricMerkleCheckRoot { root, .. } => root,
            StateReturn::MapMember { key, .. } | StateReturn::MapLookup { key, .. } => key,
            _ => panic!("fixture"),
        };
        let Expr::Add { left, right } = original else {
            panic!("fixture")
        };
        let trace = Trace::default();
        error(
            &circuit,
            &HashMap::from([("crate", &target)]),
            &trace,
            RenderError::InvalidFieldLiteral("bad".into()),
            17,
        );
        assert_eq!(
            trace.0.borrow().first(),
            Some(&key(Owner::Expr(left), Slot::Node))
        );
        assert!(
            trace
                .0
                .borrow()
                .iter()
                .all(|row| row.0 == "expr" && row.1 != std::ptr::from_ref(right.as_ref()) as usize)
        );
    }
}

#[test]
fn successful_all_nineteen_return_views_preserve_tokens_seed_and_effect_state() {
    for (target, circuit) in cases("slot") {
        let trace = Trace::default();
        let fields = HashMap::from([("slot", &target)]);
        let observed = run(&circuit, &fields, Some(&trace));
        assert!(observed.0.is_ok());
        assert_eq!(observed, run(&circuit, &fields, None));
        assert_eq!((observed.2, observed.3, observed.4), (17, false, 4));
        assert!(trace.0.borrow().is_empty());
    }
    // Effectful is the prepared tail view: its body has already been lowered by
    // the parent traversal. This does not assert that this function checks it.
    for circuit in [
        source(Type::Unit, StateReturn::Unit),
        source(
            Type::Field,
            StateReturn::Expression {
                value: literal("1"),
            },
        ),
        source(
            Type::Boolean,
            StateReturn::Effectful {
                body: ReturnPlan::Value {
                    value: Expr::Boolean { value: true },
                },
            },
        ),
    ] {
        let trace = Trace::default();
        let observed = run(&circuit, &HashMap::new(), Some(&trace));
        assert!(observed.0.is_ok());
        assert_eq!(observed, run(&circuit, &HashMap::new(), None));
        assert_eq!((observed.2, observed.3, observed.4), (17, false, 1));
        assert!(trace.0.borrow().is_empty());
    }
}

#[test]
fn existing_expression_observations_and_partial_temporaries_remain_unchanged() {
    let circuit = source(
        Type::Boolean,
        StateReturn::Expression {
            value: Expr::Add {
                left: Box::new(literal("1")),
                right: Box::new(literal("2")),
            },
        },
    );
    let StateReturn::Expression { value, .. } = &circuit.return_value else {
        panic!("fixture")
    };
    let trace = Trace::default();
    error(
        &circuit,
        &HashMap::new(),
        &trace,
        RenderError::TypeMismatch {
            expected: Type::Boolean,
            actual: Type::Field,
        },
        19,
    );
    one(&trace, Owner::Expr(value), Slot::Node);
    let circuit = source(
        Type::Boolean,
        StateReturn::Expression {
            value: Expr::Add {
                left: Box::new(literal("1")),
                right: Box::new(literal("bad")),
            },
        },
    );
    let StateReturn::Expression {
        value: Expr::Add { right, .. },
        ..
    } = &circuit.return_value
    else {
        panic!("fixture")
    };
    let trace = Trace::default();
    error(
        &circuit,
        &HashMap::new(),
        &trace,
        RenderError::InvalidFieldLiteral("bad".into()),
        18,
    );
    one(&trace, Owner::Expr(right), Slot::Node);
}

#[test]
fn equal_live_return_result_and_operand_occurrences_keep_distinct_original_identity() {
    let first = source(
        Type::Field,
        StateReturn::CellRead {
            field: "absent".into(),
            index: 0,
        },
    );
    let sources = [first.clone(), first];
    let mut ids = Vec::new();
    for circuit in &sources {
        let trace = Trace::default();
        error(
            circuit,
            &HashMap::new(),
            &trace,
            RenderError::UnknownLedgerField("absent".into()),
            17,
        );
        one(
            &trace,
            Owner::StateReturn(&circuit.return_value),
            Slot::Field("field"),
        );
        ids.push(key(
            Owner::StateReturn(&circuit.return_value),
            Slot::Field("field"),
        ));
    }
    assert_ne!(ids[0], ids[1]);
    let first = source(
        Type::Boolean,
        StateReturn::CounterRead {
            field: "count".into(),
            index: 0,
        },
    );
    let sources = [first.clone(), first];
    let mut ids = Vec::new();
    let target = ledger("count", LedgerFieldKind::Counter);
    for circuit in &sources {
        let trace = Trace::default();
        error(
            circuit,
            &HashMap::from([("count", &target)]),
            &trace,
            RenderError::TypeMismatch {
                expected: uint64(),
                actual: Type::Boolean,
            },
            17,
        );
        one(&trace, Owner::Type(&circuit.result), Slot::Node);
        ids.push(key(Owner::Type(&circuit.result), Slot::Node));
    }
    assert_ne!(ids[0], ids[1]);
    let first = source(
        Type::Boolean,
        StateReturn::SetMember {
            field: "set".into(),
            index: 0,
            value: Expr::Boolean { value: true },
        },
    );
    let sources = [first.clone(), first];
    let mut ids = Vec::new();
    let target = ledger("set", LedgerFieldKind::Set { ty: Type::Field });
    for circuit in &sources {
        let StateReturn::SetMember { value, .. } = &circuit.return_value else {
            panic!("fixture")
        };
        let trace = Trace::default();
        error(
            circuit,
            &HashMap::from([("set", &target)]),
            &trace,
            RenderError::TypeMismatch {
                expected: Type::Field,
                actual: Type::Boolean,
            },
            17,
        );
        one(&trace, Owner::Expr(value), Slot::Node);
        ids.push(key(Owner::Expr(value), Slot::Node));
    }
    assert_ne!(ids[0], ids[1]);
}
