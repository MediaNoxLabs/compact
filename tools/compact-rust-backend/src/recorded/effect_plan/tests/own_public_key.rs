// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Configured-key effects have builtin authority and ordered private observations.
use super::super::type_environment::TypeEnvironment;
use super::*;
use quote::ToTokens;
#[path = "own_public_key/inclusion.rs"]
mod inclusion;

fn key_type() -> Type {
    NativeWitnessBuiltin::OwnPublicKey.result_type()
}
fn key() -> Expr {
    Expr::NativeWitnessCall {
        builtin: NativeWitnessBuiltin::OwnPublicKey,
    }
}
fn bytes(value: Expr) -> Expr {
    Expr::StructField {
        value: Box::new(value),
        field: "bytes".into(),
        index: 0,
    }
}
fn root(value: Expr) -> StatefulCircuit {
    StatefulCircuit {
        source: None,
        name: "key_bytes".into(),
        internal: false,
        parameters: vec![],
        actions: vec![],
        result: Type::Bytes { length: 32 },
        return_value: StateReturn::Expression { value },
    }
}
fn authority(source: &StatefulCircuit) -> declarations::Declarations {
    let empty = HashMap::new();
    let types = TypeEnvironment::from_sources(
        source,
        &empty,
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
    )
    .unwrap();
    declarations::Declarations::new(&empty, &HashMap::new())
        .with_type_environment(types)
        .with_root(source)
}
fn projected() -> (StatefulCircuit, Plan) {
    let source = root(bytes(key()));
    let checked = check(&source, &HashMap::new(), &HashMap::new()).unwrap();
    (source, checked.plan().clone())
}

#[test]
fn builtin_carrier_is_collected_without_any_explicit_key_type_annotation() {
    let source = root(bytes(key()));
    let checked = check(&source, &HashMap::new(), &HashMap::new()).unwrap();
    let bindings = &checked.plan().body.bindings;
    assert!(matches!(bindings[0].operation, Operation::OwnPublicKey));
    assert_eq!(bindings[0].value.ty, key_type());
    assert!(matches!(
        bindings[1].operation,
        Operation::StructProject { .. }
    ));
    let rendered = render(&checked).unwrap();
    let statements = rendered
        .steps
        .iter()
        .map(|s| s.to_token_stream().to_string())
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(statements.matches("own_coin_public_key").count(), 1);
    assert!(statements.contains("FixedBytes :: new"));
}

#[test]
fn discarded_and_repeated_key_effects_remain_ordered_and_distinct() {
    let mut source = root(bytes(key()));
    source.actions = vec![StateAction::NativeWitnessCall {
        builtin: NativeWitnessBuiltin::OwnPublicKey,
    }];
    let checked = check(&source, &HashMap::new(), &HashMap::new()).unwrap();
    let bindings = &checked.plan().body.bindings;
    assert!(matches!(bindings[0].operation, Operation::OwnPublicKey));
    assert_eq!(bindings[0].path, "actions[0]");
    assert!(matches!(bindings[1].operation, Operation::OwnPublicKey));
    assert_ne!(bindings[0].value.id, bindings[1].value.id);
    let rendered = render(&checked).unwrap();
    let statements = rendered
        .steps
        .iter()
        .map(|s| s.to_token_stream().to_string())
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(statements.matches("own_coin_public_key").count(), 2);
}

#[test]
fn witness_order_and_unselected_key_branch_are_preserved_in_the_plan() {
    let declaration = WitnessDeclaration {
        source: None,
        name: "mark".into(),
        parameters: vec![],
        result: Type::Field,
    };
    let witness = || Expr::WitnessCall {
        name: "mark".into(),
        arguments: vec![],
    };
    let selected_key = Expr::If {
        condition: Box::new(Expr::Boolean { value: false }),
        then: Box::new(key()),
        otherwise: Box::new(Expr::Default { ty: key_type() }),
    };
    let mut source = root(bytes(selected_key));
    source.actions = vec![
        StateAction::Expression { value: witness() },
        StateAction::NativeWitnessCall {
            builtin: NativeWitnessBuiltin::OwnPublicKey,
        },
        StateAction::Expression { value: witness() },
    ];
    let checked = check(
        &source,
        &HashMap::new(),
        &HashMap::from([("mark", &declaration)]),
    )
    .unwrap();
    let bindings = &checked.plan().body.bindings;
    assert!(matches!(bindings[0].operation, Operation::Witness { .. }));
    assert!(matches!(bindings[1].operation, Operation::OwnPublicKey));
    assert!(matches!(bindings[2].operation, Operation::Witness { .. }));
    let Operation::If {
        then, otherwise, ..
    } = &bindings[4].operation
    else {
        panic!("branch must remain scoped")
    };
    assert!(
        then.bindings
            .iter()
            .any(|b| matches!(b.operation, Operation::OwnPublicKey))
    );
    assert!(
        !otherwise
            .bindings
            .iter()
            .any(|b| matches!(b.operation, Operation::OwnPublicKey))
    );
    render(&checked).unwrap();
}

#[test]
fn forged_result_annotations_and_escaping_ssa_do_not_replace_builtin_authority() {
    let (source, plan) = projected();
    for mode in 0..5 {
        let mut bad = plan.clone();
        match mode {
            0 => bad.body.bindings[0].value.ty = Type::Bytes { length: 32 },
            1 => {
                let Type::Struct { fields, .. } = &mut bad.body.bindings[0].value.ty else {
                    unreachable!()
                };
                fields[0].ty = Type::Bytes { length: 31 };
            }
            2 => bad.body.bindings[1].value.id = bad.body.bindings[0].value.id,
            3 => {
                let Operation::StructProject { value, .. } = &mut bad.body.bindings[1].operation
                else {
                    unreachable!()
                };
                value.id = 999;
            }
            4 => bad.body.result.id = 999,
            _ => unreachable!(),
        }
        assert!(
            validate::check(bad, authority(&source)).is_err(),
            "forged case {mode}"
        );
    }
}

#[test]
fn declared_lookalike_cannot_redefine_the_zero_operand_signature() {
    let (mut source, mut plan) = projected();
    let mut other = key_type();
    let Type::Struct { name, .. } = &mut other else {
        unreachable!()
    };
    *name = "OtherWalletKey".into();
    source.parameters.push(Parameter {
        name: "unused".into(),
        ty: other.clone(),
    });
    // The independent source environment genuinely declares this other type;
    // rejection must be the builtin signature, not merely an unknown name.
    let env = authority(&source);
    plan.parameters = vec![Value {
        id: 100,
        ty: other.clone(),
    }];
    plan.body.bindings[0].value.ty = other.clone();
    let Operation::StructProject { value, .. } = &mut plan.body.bindings[1].operation else {
        unreachable!()
    };
    value.ty = other;
    let gap = validate::check(plan, env).unwrap_err();
    assert_eq!(gap.path, "return_value.value.value");
    assert_eq!(gap.detail, "operation input/output types are incompatible");
}

#[test]
fn explicit_same_named_conflicts_are_rejected_during_bounded_source_collection() {
    let mut source = root(bytes(key()));
    let mut forged = key_type();
    let Type::Struct { fields, .. } = &mut forged else {
        unreachable!()
    };
    fields[0].ty = Type::Boolean;
    source.parameters.push(Parameter {
        name: "forged".into(),
        ty: forged,
    });
    assert!(
        matches!(check(&source, &HashMap::new(), &HashMap::new()), Err(PlanError::Unsupported(gap)) if gap.path == "types")
    );
    // A key carrier absent from source cannot be forged into a plan either.
    let (source, plan) = projected();
    let mut without_builtin = source;
    without_builtin.return_value = StateReturn::Expression {
        value: Expr::BytesLiteral { bytes: vec![0; 32] },
    };
    assert!(validate::check(plan, authority(&without_builtin)).is_err());
}

#[test]
fn repeated_private_effects_still_consume_the_normal_expansion_budget() {
    let (source, mut plan) = projected();
    let limit = crate::resource_limits::recording_call_limits().1;
    plan.body.bindings = (0..=limit)
        .map(|id| Binding {
            value: Value { id, ty: key_type() },
            operation: Operation::OwnPublicKey,
            path: "repeated-key".into(),
        })
        .collect();
    let gap = validate::check(plan, authority(&source)).unwrap_err();
    assert_eq!(gap.detail, "recording plan expanded work limit exceeded");
}

#[test]
fn discarded_only_key_action_closes_generated_type_dependencies() {
    let mut source = root(Expr::Unit);
    source.name = "discard_key".into();
    source.result = Type::Unit;
    source.return_value = StateReturn::Unit;
    source.actions = vec![StateAction::NativeWitnessCall {
        builtin: NativeWitnessBuiltin::OwnPublicKey,
    }];
    let checked = check(&source, &HashMap::new(), &HashMap::new()).unwrap();
    assert!(matches!(
        checked.plan().body.bindings[0].operation,
        Operation::OwnPublicKey
    ));
    let contract = Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        ledger_fields: vec![],
        constructor: None,
        witnesses: vec![],
        circuits: vec![],
        stateful_circuits: vec![source],
    };
    let (output, rows) = crate::recorded::admission_probe::trace::capture_json(|| {
        crate::render_with_capabilities(&contract)
    });
    let output = output.unwrap();
    assert!(output.capabilities.circuits[0].recorded);
    assert!(output.source.contains("pub struct ZswapCoinPublicKey"));
    let public: Vec<_> = rows
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| {
            r["circuit"] == "discard_key" && r["phase"] == "public_emission" && r["helper"] == false
        })
        .collect();
    assert_eq!(public.len(), 1);
    assert!(
        public[0]["body_owner"]
            .as_str()
            .unwrap()
            .starts_with("algebra.")
    );
    if let Some(base) = std::env::var_os("COMPACT_OWN_PUBLIC_KEY_PROBE_OUTPUT") {
        let base = std::path::PathBuf::from(base);
        std::fs::create_dir_all(&base).unwrap();
        let dir = base.join("discarded-only");
        std::fs::create_dir(&dir).expect("use fresh evidence directory");
        std::fs::write(
            dir.join("compact-rust-ir.json"),
            serde_json::to_vec_pretty(&contract).unwrap(),
        )
        .unwrap();
        std::fs::write(dir.join("lib.rs"), output.source).unwrap();
    }
}
