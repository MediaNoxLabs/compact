// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

#[path = "tests/calls.rs"]
mod calls;
#[path = "tests/context_queries.rs"]
mod context_queries;
#[path = "tests/counter.rs"]
mod counter;
#[path = "tests/declarations.rs"]
mod declaration_tests;
#[path = "tests/immediate_send_inclusion.rs"]
mod immediate_send_inclusion;
#[path = "tests/jubjub.rs"]
mod jubjub;
#[path = "tests/kernel.rs"]
mod kernel;
#[path = "tests/ordinary.rs"]
mod ordinary;
#[path = "tests/phase_reset_inclusion.rs"]
mod phase_reset_inclusion;
#[path = "tests/pure_inclusion.rs"]
mod pure_inclusion;
#[path = "tests/pure_validation.rs"]
mod pure_validation;
#[path = "tests/rich_validation.rs"]
mod rich_validation;
#[path = "tests/rich_values.rs"]
mod rich_values;
#[path = "tests/tuple_equality.rs"]
mod tuple_equality;
#[path = "tests/wide_values.rs"]
mod wide_values;
#[path = "tests/zswap_inclusion.rs"]
mod zswap_inclusion;
use declaration_tests::check_plan;

use super::model::*;
use super::*;
use crate::ir::*;
fn field(n: usize) -> Value {
    Value {
        id: n,
        ty: Type::Field,
    }
}
fn literal(id: usize) -> Binding {
    Binding {
        value: field(id),
        operation: Operation::Literal(Literal::Field([0; 32])),
        path: format!("steps[{id}]"),
    }
}
fn plan(bindings: Vec<Binding>, result: Value) -> Plan {
    Plan {
        parameters: vec![],
        body: Block { bindings, result },
        result_type: Type::Field,
    }
}
#[test]
fn no_effect_count_is_required() {
    assert!(check_plan(plan(vec![literal(0)], field(0))).is_ok());
}
#[test]
fn validator_rejects_use_before_definition_and_duplicate_ids() {
    let forward = Binding {
        value: field(0),
        operation: Operation::Add(field(1), field(1)),
        path: "first".into(),
    };
    assert_eq!(
        check_plan(plan(vec![forward, literal(1)], field(0)))
            .unwrap_err()
            .path,
        "first"
    );
    assert!(check_plan(plan(vec![literal(0), literal(0)], field(0))).is_err());
}
#[test]
fn validator_rejects_branch_scope_escape_and_mismatched_join() {
    let condition = Value {
        id: 0,
        ty: Type::Boolean,
    };
    let cond = Binding {
        value: condition.clone(),
        operation: Operation::Literal(Literal::Boolean(true)),
        path: "condition".into(),
    };
    let branch = Binding {
        value: field(3),
        operation: Operation::If {
            condition,
            then: Block {
                bindings: vec![literal(1)],
                result: field(1),
            },
            otherwise: Block {
                bindings: vec![literal(2)],
                result: field(2),
            },
        },
        path: "branch".into(),
    };
    assert!(check_plan(plan(vec![cond.clone(), branch.clone()], field(1))).is_err());
    let mut bad = branch;
    if let Operation::If { otherwise, .. } = &mut bad.operation {
        otherwise.result = Value {
            id: 0,
            ty: Type::Boolean,
        };
    }
    assert!(check_plan(plan(vec![cond, bad], field(3))).is_err());
}
#[test]
fn unsupported_children_report_first_source_path() {
    let circuit = StatefulCircuit {
        source: None,
        name: "test".into(),
        internal: false,
        parameters: vec![],
        actions: vec![],
        result: Type::Field,
        return_value: StateReturn::Effectful {
            body: ReturnPlan::Conditional {
                condition: Expr::Boolean { value: true },
                then: Box::new(ReturnPlan::Value {
                    value: Expr::Call {
                        name: "unsupported".into(),
                        arguments: vec![],
                    },
                }),
                otherwise: Box::new(ReturnPlan::Value {
                    value: Expr::BytesLiteral { bytes: vec![0] },
                }),
            },
        },
    };
    let PlanError::Unsupported(gap) =
        check(&circuit, &HashMap::new(), &HashMap::new()).unwrap_err()
    else {
        panic!("expected unsupported source node")
    };
    assert_eq!(gap.path, "return_value.body.then.value");
    assert_eq!(gap.ir_node, "Expr::Call");
}
#[test]
fn narrowing_coercion_is_rejected_but_explicit_cast_is_checked() {
    let input = Value {
        id: 0,
        ty: validate::uint("65535"),
    };
    let output = Value {
        id: 1,
        ty: validate::uint("255"),
    };
    let candidate = |kind| Plan {
        parameters: vec![input.clone()],
        body: Block {
            bindings: vec![Binding {
                value: output.clone(),
                operation: Operation::Convert {
                    value: input.clone(),
                    kind,
                },
                path: "cast".into(),
            }],
            result: output.clone(),
        },
        result_type: output.ty.clone(),
    };
    assert!(check_plan(candidate(Conversion::Coerce)).is_err());
    let checked = check_plan(candidate(Conversion::UnsignedCast)).unwrap();
    let rendered = render(&checked).unwrap();
    let steps = rendered.steps;
    assert!(
        quote::quote!(#(#steps)*)
            .to_string()
            .contains("cast_unsigned")
    );
}

#[test]
fn operation_contracts_reject_incompatible_inputs() {
    let field_value = field(0);
    let boolean = Value {
        id: 1,
        ty: Type::Boolean,
    };
    let small = Value {
        id: 2,
        ty: validate::uint("255"),
    };
    let cases = [
        (
            "witness_arity",
            Type::Field,
            Operation::Witness {
                declaration: WitnessId(0),
                arguments: vec![],
            },
        ),
        (
            "witness_type",
            Type::Field,
            Operation::Witness {
                declaration: WitnessId(0),
                arguments: vec![boolean.clone()],
            },
        ),
        (
            "cell_value",
            Type::Unit,
            Operation::CellWrite {
                slot: CellSlot {
                    declaration: LedgerId(1),
                    index: 0,
                },
                value: boolean.clone(),
            },
        ),
        (
            "counter_amount",
            Type::Unit,
            Operation::CounterIncrement {
                slot: CounterSlot {
                    declaration: LedgerId(0),
                    index: 1,
                },
                amount: small.clone(),
            },
        ),
        (
            "assert_condition",
            Type::Unit,
            Operation::Assert {
                condition: field_value.clone(),
                message: "invalid".into(),
            },
        ),
        (
            "addition",
            Type::Field,
            Operation::Add(field_value.clone(), boolean.clone()),
        ),
    ];
    for (path, ty, operation) in cases {
        let output = Value {
            id: 3,
            ty: ty.clone(),
        };
        let candidate = Plan {
            parameters: vec![field_value.clone(), boolean.clone(), small.clone()],
            body: Block {
                bindings: vec![Binding {
                    value: output.clone(),
                    operation,
                    path: path.into(),
                }],
                result: output,
            },
            result_type: ty,
        };
        assert_eq!(check_plan(candidate).unwrap_err().path, path);
    }
}

#[test]
fn value_ids_are_unique_across_sibling_branches() {
    let condition = Value {
        id: 0,
        ty: Type::Boolean,
    };
    let candidate = Plan {
        parameters: vec![condition.clone()],
        body: Block {
            bindings: vec![Binding {
                value: field(2),
                path: "join".into(),
                operation: Operation::If {
                    condition,
                    then: Block {
                        bindings: vec![literal(1)],
                        result: field(1),
                    },
                    otherwise: Block {
                        bindings: vec![literal(1)],
                        result: field(1),
                    },
                },
            }],
            result: field(2),
        },
        result_type: Type::Field,
    };
    assert_eq!(check_plan(candidate).unwrap_err().path, "steps[1]");
}

#[test]
fn statically_unselected_branch_is_checked_with_real_expression_path() {
    let circuit = StatefulCircuit {
        source: None,
        name: "test".into(),
        internal: false,
        parameters: vec![],
        actions: vec![],
        result: Type::Field,
        return_value: StateReturn::Effectful {
            body: ReturnPlan::Value {
                value: Expr::If {
                    condition: Box::new(Expr::Boolean { value: true }),
                    then: Box::new(Expr::FieldLiteral { value: "1".into() }),
                    otherwise: Box::new(Expr::Call {
                        name: "unsupported".into(),
                        arguments: vec![],
                    }),
                },
            },
        },
    };
    for _ in 0..2 {
        let PlanError::Unsupported(gap) =
            check(&circuit, &HashMap::new(), &HashMap::new()).unwrap_err()
        else {
            panic!("expected fragment refusal")
        };
        assert_eq!(gap.path, "return_value.body.value.otherwise");
    }
}

#[test]
fn actual_compiler_compositions_lower_without_profile_counts() {
    let contract: Contract = serde_json::from_str(include_str!(
        "../../../tests/effect-plan-prototype-schema20-ir.json"
    ))
    .unwrap();
    let ledger = contract
        .ledger_fields
        .iter()
        .map(|f| (f.id.as_str(), f))
        .collect();
    let witnesses = contract
        .witnesses
        .iter()
        .map(|w| (w.name.as_str(), w))
        .collect();
    assert_eq!(contract.stateful_circuits.len(), 2);
    for circuit in &contract.stateful_circuits {
        let checked = check(circuit, &ledger, &witnesses).unwrap();
        assert_eq!(checked.plan().body.result.ty, Type::Field);
        let rendered = render(&checked).unwrap();
        let steps = rendered.steps;
        let result = rendered.result;
        let _: syn::Block = syn::parse_quote!({ #(#steps)* #result });
    }
}

#[test]
fn small_unsigned_literal_cast_and_add_remain_checked_operations() {
    let small = Value {
        id: 0,
        ty: validate::uint("255"),
    };
    let widened = Value {
        id: 1,
        ty: validate::uint("65535"),
    };
    let sum = Value {
        id: 2,
        ty: validate::uint("65535"),
    };
    let output = field(3);
    let candidate = Plan {
        parameters: vec![],
        body: Block {
            bindings: vec![
                Binding {
                    value: small.clone(),
                    operation: Operation::Literal(Literal::Unsigned(255)),
                    path: "literal".into(),
                },
                Binding {
                    value: widened.clone(),
                    operation: Operation::Convert {
                        value: small,
                        kind: Conversion::Coerce,
                    },
                    path: "widen".into(),
                },
                Binding {
                    value: sum.clone(),
                    operation: Operation::UnsignedAdd(widened.clone(), widened),
                    path: "add".into(),
                },
                Binding {
                    value: output.clone(),
                    operation: Operation::Convert {
                        value: sum,
                        kind: Conversion::Coerce,
                    },
                    path: "field".into(),
                },
            ],
            result: output,
        },
        result_type: Type::Field,
    };
    let rendered = render(&check_plan(candidate).unwrap()).unwrap();
    let steps = rendered.steps;
    let source = quote::quote!(#(#steps)*).to_string();
    assert!(source.contains("add_unsigned"));
    assert!(source.contains("cast_unsigned"));
    let too_large = Value {
        id: 0,
        ty: validate::uint("255"),
    };
    let invalid = Plan {
        parameters: vec![],
        body: Block {
            bindings: vec![Binding {
                value: too_large.clone(),
                operation: Operation::Literal(Literal::Unsigned(256)),
                path: "literal".into(),
            }],
            result: too_large.clone(),
        },
        result_type: too_large.ty,
    };
    assert_eq!(check_plan(invalid).unwrap_err().path, "literal");
}

mod call_validation;

mod composite;

mod owned_values;

mod receive_owned_domain_probes;

#[path = "tests/counter_hash.rs"]
mod counter_hash;
#[path = "tests/counter_hash_inclusion.rs"]
mod counter_hash_inclusion;

#[path = "tests/owned_cells.rs"]
mod owned_cells;

#[path = "tests/set_operations.rs"]
mod set_operations;

#[path = "tests/unsigned_arithmetic.rs"]
mod unsigned_arithmetic;

#[path = "tests/transient_crypto.rs"]
mod transient_crypto;

#[path = "tests/merkle.rs"]
mod merkle;

#[path = "tests/witness_results.rs"]
mod witness_results;

#[path = "tests/owned_calls.rs"]
mod owned_calls;

#[path = "tests/qualified_coin_set.rs"]
mod qualified_coin_set;
#[path = "tests/qualified_coin_write.rs"]
mod qualified_coin_write;

#[path = "tests/opaque_public_probe.rs"]
mod opaque_public_probe;
