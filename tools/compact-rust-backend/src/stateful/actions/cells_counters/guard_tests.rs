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

//! Exact borrowed guard identity and preserved competing-error order in both lanes.
use super::*;
use crate::ir::{ConstructorStep, Expr};
use crate::source_diagnostics::Slot;
use std::cell::RefCell;

#[derive(Default)]
struct Trace(RefCell<Vec<(&'static str, usize, Slot)>>);
fn key(owner: Owner<'_>, slot: Slot) -> (&'static str, usize, Slot) {
    let (kind, pointer) = match owner {
        Owner::Action(value) => ("action", std::ptr::from_ref(value) as usize),
        Owner::ConstructorStep(value) => ("step", std::ptr::from_ref(value) as usize),
        Owner::CounterAmount(value) => ("amount", std::ptr::from_ref(value) as usize),
        Owner::Expr(value) => ("expr", std::ptr::from_ref(value) as usize),
        Owner::Ledger(value) => ("ledger", std::ptr::from_ref(value) as usize),
        _ => panic!("unexpected guard source owner"),
    };
    (kind, pointer, slot)
}
impl RefusalObserver for Trace {
    fn refused(&self, site: RefusalSite<'_>) {
        self.0
            .borrow_mut()
            .push(key(site.owner.expect("original owner"), site.slot));
    }
}
fn one(trace: &Trace, owner: Owner<'_>, slot: Slot) {
    assert_eq!(trace.0.borrow().as_slice(), &[key(owner, slot)]);
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
fn amount(name: &str) -> CounterAmount {
    CounterAmount::Parameter { name: name.into() }
}
fn field(value: &str) -> Expr {
    Expr::FieldLiteral {
        value: value.into(),
    }
}
fn unsigned16() -> Type {
    Type::Unsigned {
        max: "65535".into(),
    }
}
fn action_run(
    action: &StateAction,
    fields: &HashMap<&str, &LedgerField>,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    observer: Option<&dyn RefusalObserver>,
) -> (Result<(), RenderError>, String, usize, bool) {
    let witnesses = HashMap::new();
    let circuits = HashMap::new();
    let stateful_circuits = HashMap::new();
    let environment = NativeExpressionContext {
        witnesses: &witnesses,
        circuits: &circuits,
        stateful_circuits: &stateful_circuits,
        ledger_fields: fields,
        observer,
    };
    let mut statements = Vec::new();
    let mut next_temp = 9;
    let mut uses_witness = false;
    let result = if observer.is_some() {
        lower_observed(
            action,
            parameters,
            &environment,
            ActionOutput {
                statements: &mut statements,
                next_temp: &mut next_temp,
                uses_witness: &mut uses_witness,
            },
        )
    } else {
        lower(
            action,
            parameters,
            fields,
            &witnesses,
            &circuits,
            &stateful_circuits,
            &mut statements,
            &mut next_temp,
            &mut uses_witness,
        )
    };
    (
        result,
        quote::quote!(#(#statements)*).to_string(),
        next_temp,
        uses_witness,
    )
}
fn vm_run<'a>(
    step: &'a ConstructorStep,
    fields: &HashMap<&str, &LedgerField>,
    parameters: &HashMap<&'a str, (&'a Type, syn::Ident)>,
    observer: Option<&dyn RefusalObserver>,
) -> (Result<String, RenderError>, usize, usize) {
    let witnesses = HashMap::new();
    let circuits = HashMap::new();
    let stateful_circuits = HashMap::new();
    let environment = NativeExpressionContext {
        witnesses: &witnesses,
        circuits: &circuits,
        stateful_circuits: &stateful_circuits,
        ledger_fields: fields,
        observer,
    };
    let mut next_loop = 7;
    let mut next_temp = 9;
    let result = if observer.is_some() {
        crate::render_constructor_vm_steps_observed(
            std::slice::from_ref(step),
            parameters,
            &environment,
            crate::constructor_vm::Output {
                next_loop: &mut next_loop,
                next_temp: &mut next_temp,
            },
        )
    } else {
        crate::render_constructor_vm_steps(
            std::slice::from_ref(step),
            fields,
            parameters,
            &witnesses,
            &circuits,
            &stateful_circuits,
            &mut next_loop,
            &mut next_temp,
        )
    }
    .map(|statements| quote::quote!(#(#statements)*).to_string());
    (result, next_loop, next_temp)
}
fn action_error(
    action: &StateAction,
    fields: &HashMap<&str, &LedgerField>,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    trace: &Trace,
    expected: RenderError,
) {
    let bare = action_run(action, fields, parameters, None);
    let observed = action_run(action, fields, parameters, Some(trace));
    assert_eq!(observed, bare, "error and partial output/temp/effect state");
    assert_eq!(observed.0, Err(expected));
}
fn vm_error<'a>(
    step: &'a ConstructorStep,
    fields: &HashMap<&str, &LedgerField>,
    parameters: &HashMap<&'a str, (&'a Type, syn::Ident)>,
    trace: &Trace,
    expected: RenderError,
) {
    let bare = vm_run(step, fields, parameters, None);
    let observed = vm_run(step, fields, parameters, Some(trace));
    assert_eq!(observed, bare, "error and loop/temp state");
    assert_eq!(observed.0, Err(expected));
}

#[test]
fn action_counter_lookup_and_combined_guard_keep_exact_original_action() {
    let target = ledger("count", LedgerFieldKind::Cell { ty: Type::Field });
    let fields = HashMap::from([("count", &target)]);
    for action in [
        StateAction::CounterIncrement {
            field: "absent".into(),
            index: 1,
            amount: amount("later"),
        },
        StateAction::CounterDecrement {
            field: "count".into(),
            index: 1,
            amount: amount("later"),
        },
        StateAction::CounterReset {
            field: "count".into(),
            index: 1,
        },
        StateAction::CounterReset {
            field: "absent".into(),
            index: 0,
        },
    ] {
        let (StateAction::CounterIncrement { field, .. }
        | StateAction::CounterDecrement { field, .. }
        | StateAction::CounterReset { field, .. }) = &action
        else {
            panic!("fixture")
        };
        let (name, slot) = if field == "absent" {
            ("absent", Slot::Field("field"))
        } else {
            ("count", Slot::Node)
        };
        let trace = Trace::default();
        action_error(
            &action,
            &fields,
            &HashMap::new(),
            &trace,
            RenderError::UnknownLedgerField(name.into()),
        );
        one(&trace, Owner::Action(&action), slot);
    }
}

#[test]
fn action_cell_kind_precedes_index_and_type_precedes_identifier() {
    let wrong = ledger("crate", LedgerFieldKind::Counter);
    let cell = ledger("crate", LedgerFieldKind::Cell { ty: Type::Field });
    let action = StateAction::CellWrite {
        field: "crate".into(),
        index: 1,
        value: Expr::Parameter {
            name: "later".into(),
        },
    };
    let trace = Trace::default();
    action_error(
        &action,
        &HashMap::from([("crate", &wrong)]),
        &HashMap::new(),
        &trace,
        RenderError::UnknownLedgerField("crate".into()),
    );
    one(&trace, Owner::Action(&action), Slot::Field("field"));
    let trace = Trace::default();
    action_error(
        &action,
        &HashMap::from([("crate", &cell)]),
        &HashMap::new(),
        &trace,
        RenderError::UnknownLedgerField("crate".into()),
    );
    one(&trace, Owner::Action(&action), Slot::Field("index"));
    let missing = StateAction::CellWrite {
        field: "absent".into(),
        index: 1,
        value: field("bad"),
    };
    let trace = Trace::default();
    action_error(
        &missing,
        &HashMap::new(),
        &HashMap::new(),
        &trace,
        RenderError::UnknownLedgerField("absent".into()),
    );
    one(&trace, Owner::Action(&missing), Slot::Field("field"));
    let mismatch = StateAction::CellWrite {
        field: "crate".into(),
        index: 0,
        value: Expr::Boolean { value: true },
    };
    let StateAction::CellWrite { value, .. } = &mismatch else {
        panic!("fixture")
    };
    let trace = Trace::default();
    action_error(
        &mismatch,
        &HashMap::from([("crate", &cell)]),
        &HashMap::new(),
        &trace,
        RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean,
        },
    );
    one(&trace, Owner::Expr(value), Slot::Node);
}

#[test]
fn counter_amount_raw_lookup_and_type_keep_each_original_occurrence_in_both_lanes() {
    let target = ledger("count", LedgerFieldKind::Counter);
    let fields = HashMap::from([("count", &target)]);
    let wrong_type = Type::Boolean;
    let parameters = HashMap::from([(
        "r#raw$",
        (&wrong_type, syn::Ident::new("input", Span::call_site())),
    )]);
    for action in [
        StateAction::CounterIncrement {
            field: "count".into(),
            index: 0,
            amount: amount("missing"),
        },
        StateAction::CounterDecrement {
            field: "count".into(),
            index: 0,
            amount: amount("r#raw$"),
        },
        StateAction::CounterIncrement {
            field: "count".into(),
            index: 0,
            amount: amount("r#raw$"),
        },
    ] {
        let (StateAction::CounterIncrement {
            amount: original, ..
        }
        | StateAction::CounterDecrement {
            amount: original, ..
        }) = &action
        else {
            panic!("fixture")
        };
        let CounterAmount::Parameter { name } = original else {
            panic!("fixture")
        };
        let expected_name = name.clone();
        let expected = || {
            if expected_name == "missing" {
                RenderError::UnknownParameter(expected_name.clone())
            } else {
                RenderError::TypeMismatch {
                    expected: unsigned16(),
                    actual: Type::Boolean,
                }
            }
        };
        let trace = Trace::default();
        action_error(&action, &fields, &parameters, &trace, expected());
        one(&trace, Owner::CounterAmount(original), Slot::Field("name"));
        let step = match action {
            StateAction::CounterIncrement {
                field,
                index,
                amount,
            } => ConstructorStep::CounterIncrement {
                field,
                index,
                amount,
            },
            StateAction::CounterDecrement {
                field,
                index,
                amount,
            } => ConstructorStep::CounterDecrement {
                field,
                index,
                amount,
            },
            _ => unreachable!(),
        };
        let (ConstructorStep::CounterIncrement {
            amount: original, ..
        }
        | ConstructorStep::CounterDecrement {
            amount: original, ..
        }) = &step
        else {
            panic!("fixture")
        };
        let trace = Trace::default();
        vm_error(&step, &fields, &parameters, &trace, expected());
        one(&trace, Owner::CounterAmount(original), Slot::Field("name"));
    }
}

#[test]
fn action_identifier_wins_over_amount_but_vm_has_no_identifier_check() {
    let target = ledger("crate", LedgerFieldKind::Counter);
    let fields = HashMap::from([("crate", &target)]);
    let action = StateAction::CounterIncrement {
        field: "crate".into(),
        index: 0,
        amount: amount("missing"),
    };
    let trace = Trace::default();
    action_error(
        &action,
        &fields,
        &HashMap::new(),
        &trace,
        RenderError::InvalidIdentifier("crate".into()),
    );
    one(&trace, Owner::Ledger(&target), Slot::Field("id"));
    let step = ConstructorStep::CounterIncrement {
        field: "crate".into(),
        index: 0,
        amount: amount("missing"),
    };
    let ConstructorStep::CounterIncrement { amount, .. } = &step else {
        panic!("fixture")
    };
    let trace = Trace::default();
    vm_error(
        &step,
        &fields,
        &HashMap::new(),
        &trace,
        RenderError::UnknownParameter("missing".into()),
    );
    one(&trace, Owner::CounterAmount(amount), Slot::Field("name"));
    let valid = ConstructorStep::CounterIncrement {
        field: "crate".into(),
        index: 0,
        amount: CounterAmount::Literal { value: 3 },
    };
    let trace = Trace::default();
    let observed = vm_run(&valid, &fields, &HashMap::new(), Some(&trace));
    assert!(observed.0.is_ok());
    assert_eq!(observed, vm_run(&valid, &fields, &HashMap::new(), None));
    assert!(trace.0.borrow().is_empty());
}

#[test]
fn vm_cell_lookup_then_index_then_kind_precede_operand_and_outer_type() {
    let wrong = ledger("stored", LedgerFieldKind::Counter);
    for (name, index, expected, slot) in [
        (
            "absent",
            1,
            RenderError::UnknownLedgerField("absent".into()),
            Slot::Field("field"),
        ),
        (
            "stored",
            1,
            RenderError::InvalidLedgerIndex(1),
            Slot::Field("index"),
        ),
        (
            "stored",
            0,
            RenderError::InvalidConstructorInitializer("stored".into()),
            Slot::Field("field"),
        ),
    ] {
        let step = ConstructorStep::CellWrite {
            field: name.into(),
            index,
            value: field("bad"),
        };
        let trace = Trace::default();
        vm_error(
            &step,
            &HashMap::from([("stored", &wrong)]),
            &HashMap::new(),
            &trace,
            expected,
        );
        one(&trace, Owner::ConstructorStep(&step), slot);
    }
    let cell = ledger("stored", LedgerFieldKind::Cell { ty: Type::Field });
    let step = ConstructorStep::CellWrite {
        field: "stored".into(),
        index: 0,
        value: Expr::Boolean { value: true },
    };
    let ConstructorStep::CellWrite { value, .. } = &step else {
        panic!("fixture")
    };
    let trace = Trace::default();
    vm_error(
        &step,
        &HashMap::from([("stored", &cell)]),
        &HashMap::new(),
        &trace,
        RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean,
        },
    );
    one(&trace, Owner::Expr(value), Slot::Node);
}

#[test]
fn vm_counter_lookup_and_combined_target_guard_precede_amount() {
    let target = ledger("count", LedgerFieldKind::Cell { ty: Type::Field });
    let fields = HashMap::from([("count", &target)]);
    for step in [
        ConstructorStep::CounterIncrement {
            field: "absent".into(),
            index: 1,
            amount: amount("later"),
        },
        ConstructorStep::CounterDecrement {
            field: "count".into(),
            index: 1,
            amount: amount("later"),
        },
        ConstructorStep::CounterReset {
            field: "count".into(),
            index: 1,
        },
        ConstructorStep::CounterReset {
            field: "absent".into(),
            index: 0,
        },
    ] {
        let (ConstructorStep::CounterIncrement { field, .. }
        | ConstructorStep::CounterDecrement { field, .. }
        | ConstructorStep::CounterReset { field, .. }) = &step
        else {
            panic!("fixture")
        };
        let (name, slot) = if field == "absent" {
            ("absent", Slot::Field("field"))
        } else {
            ("count", Slot::Node)
        };
        let trace = Trace::default();
        vm_error(
            &step,
            &fields,
            &HashMap::new(),
            &trace,
            RenderError::UnknownLedgerField(name.into()),
        );
        one(&trace, Owner::ConstructorStep(&step), slot);
    }
    let counter = ledger("count", LedgerFieldKind::Counter);
    let index_only = ConstructorStep::CounterReset {
        field: "count".into(),
        index: 1,
    };
    let trace = Trace::default();
    vm_error(
        &index_only,
        &HashMap::from([("count", &counter)]),
        &HashMap::new(),
        &trace,
        RenderError::UnknownLedgerField("count".into()),
    );
    one(&trace, Owner::ConstructorStep(&index_only), Slot::Node);
}

#[test]
fn original_cell_child_failure_is_not_replaced_by_new_outer_type_observation() {
    let target = ledger("crate", LedgerFieldKind::Cell { ty: Type::Boolean });
    let fields = HashMap::from([("crate", &target)]);
    let action = StateAction::CellWrite {
        field: "crate".into(),
        index: 0,
        value: Expr::Add {
            left: Box::new(field("bad")),
            right: Box::new(Expr::Parameter {
                name: "later".into(),
            }),
        },
    };
    let StateAction::CellWrite { value, .. } = &action else {
        panic!("fixture")
    };
    let Expr::Add { left, .. } = value else {
        panic!("fixture")
    };
    let trace = Trace::default();
    action_error(
        &action,
        &fields,
        &HashMap::new(),
        &trace,
        RenderError::InvalidFieldLiteral("bad".into()),
    );
    one(&trace, Owner::Expr(left), Slot::Node);
    let step = ConstructorStep::CellWrite {
        field: "crate".into(),
        index: 0,
        value: value.clone(),
    };
    let ConstructorStep::CellWrite {
        value: Expr::Add { left, .. },
        ..
    } = &step
    else {
        panic!("fixture")
    };
    let trace = Trace::default();
    vm_error(
        &step,
        &fields,
        &HashMap::new(),
        &trace,
        RenderError::InvalidFieldLiteral("bad".into()),
    );
    one(&trace, Owner::Expr(left), Slot::Node);
}

#[test]
fn valid_counter_cell_both_lanes_have_identical_output_state_and_no_refusals() {
    let count = ledger("count", LedgerFieldKind::Counter);
    let cell = ledger("stored", LedgerFieldKind::Cell { ty: Type::Field });
    let fields = HashMap::from([("count", &count), ("stored", &cell)]);
    let ty = unsigned16();
    let parameters =
        HashMap::from([("r#raw$", (&ty, syn::Ident::new("input", Span::call_site())))]);
    for action in [
        StateAction::CounterIncrement {
            field: "count".into(),
            index: 0,
            amount: amount("r#raw$"),
        },
        StateAction::CounterDecrement {
            field: "count".into(),
            index: 0,
            amount: CounterAmount::Literal { value: 2 },
        },
        StateAction::CounterReset {
            field: "count".into(),
            index: 0,
        },
        StateAction::CellWrite {
            field: "stored".into(),
            index: 0,
            value: field("1"),
        },
    ] {
        let trace = Trace::default();
        let observed = action_run(&action, &fields, &parameters, Some(&trace));
        assert!(observed.0.is_ok());
        assert_eq!(observed, action_run(&action, &fields, &parameters, None));
        assert!(trace.0.borrow().is_empty());
        let step = match action {
            StateAction::CounterIncrement {
                field,
                index,
                amount,
            } => ConstructorStep::CounterIncrement {
                field,
                index,
                amount,
            },
            StateAction::CounterDecrement {
                field,
                index,
                amount,
            } => ConstructorStep::CounterDecrement {
                field,
                index,
                amount,
            },
            StateAction::CounterReset { field, index } => {
                ConstructorStep::CounterReset { field, index }
            }
            StateAction::CellWrite {
                field,
                index,
                value,
            } => ConstructorStep::CellWrite {
                field,
                index,
                value,
            },
            _ => unreachable!(),
        };
        let trace = Trace::default();
        let observed = vm_run(&step, &fields, &parameters, Some(&trace));
        assert!(observed.0.is_ok());
        assert_eq!(observed, vm_run(&step, &fields, &parameters, None));
        assert!(trace.0.borrow().is_empty());
    }
}

#[test]
fn equal_amount_names_at_distinct_live_occurrences_keep_distinct_identity() {
    let target = ledger("count", LedgerFieldKind::Counter);
    let fields = HashMap::from([("count", &target)]);
    let first = StateAction::CounterIncrement {
        field: "count".into(),
        index: 0,
        amount: amount("missing"),
    };
    let actions = [first.clone(), first];
    let mut action_sites = Vec::new();
    for action in &actions {
        let StateAction::CounterIncrement { amount, .. } = action else {
            panic!("fixture")
        };
        let trace = Trace::default();
        action_error(
            action,
            &fields,
            &HashMap::new(),
            &trace,
            RenderError::UnknownParameter("missing".into()),
        );
        one(&trace, Owner::CounterAmount(amount), Slot::Field("name"));
        action_sites.push(key(Owner::CounterAmount(amount), Slot::Field("name")));
    }
    assert_ne!(action_sites[0], action_sites[1]);
    let first = ConstructorStep::CounterIncrement {
        field: "count".into(),
        index: 0,
        amount: amount("missing"),
    };
    let steps = [first.clone(), first];
    let mut vm_sites = Vec::new();
    for step in &steps {
        let ConstructorStep::CounterIncrement { amount, .. } = step else {
            panic!("fixture")
        };
        let trace = Trace::default();
        vm_error(
            step,
            &fields,
            &HashMap::new(),
            &trace,
            RenderError::UnknownParameter("missing".into()),
        );
        one(&trace, Owner::CounterAmount(amount), Slot::Field("name"));
        vm_sites.push(key(Owner::CounterAmount(amount), Slot::Field("name")));
    }
    assert_ne!(vm_sites[0], vm_sites[1]);
}
