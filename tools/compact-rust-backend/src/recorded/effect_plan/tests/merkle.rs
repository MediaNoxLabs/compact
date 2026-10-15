// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Merkle signatures and adversarial independent-plan checks, separate from VM proofs.
use super::super::type_environment::TypeEnvironment;
use super::*;
#[path = "merkle/validation.rs"]
mod validation;

fn digest() -> Type {
    Type::Struct {
        name: "MerkleTreeDigest".into(),
        fields: vec![StructField {
            name: "field".into(),
            ty: Type::Field,
        }],
    }
}
fn sample(ty: Type) -> Expr {
    match ty {
        Type::Unit => Expr::Unit,
        Type::Boolean => Expr::Boolean { value: false },
        Type::Field => Expr::FieldLiteral { value: "0".into() },
        Type::Bytes { length } => Expr::BytesLiteral {
            bytes: vec![0; length],
        },
        Type::Unsigned { max } => Expr::UnsignedLiteral {
            value: "0".into(),
            max,
        },
        Type::JubjubPoint => Expr::EcMulGenerator {
            scalar: Box::new(Expr::JubjubScalarFromNative {
                value: Box::new(Expr::FieldLiteral { value: "0".into() }),
            }),
        },
        Type::OpaqueString | Type::OpaqueBytes => Expr::StructField {
            value: Box::new(Expr::Default {
                ty: Type::Struct {
                    name: "OpaqueLeafSource".into(),
                    fields: vec![StructField {
                        name: "value".into(),
                        ty,
                    }],
                },
            }),
            field: "value".into(),
            index: 0,
        },
        _ => Expr::Default { ty },
    }
}
fn uint64() -> Type {
    Type::Unsigned {
        max: u64::MAX.to_string(),
    }
}
fn declaration(historic: bool, depth: u8, ty: Type) -> LedgerFieldKind {
    if historic {
        LedgerFieldKind::HistoricMerkleTree { depth, ty }
    } else {
        LedgerFieldKind::MerkleTree { depth, ty }
    }
}
fn contract(historic: bool, ty: Type) -> Contract {
    Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        circuits: vec![],
        witnesses: vec![],
        ledger_fields: vec![LedgerField {
            id: "tree".into(),
            index: 0,
            path: vec![0],
            source: None,
            declaration: declaration(historic, 3, ty),
        }],
        stateful_circuits: vec![StatefulCircuit {
            name: "merkle".into(),
            source: None,
            internal: false,
            parameters: vec![],
            actions: vec![],
            result: Type::Unit,
            return_value: StateReturn::Unit,
        }],
    }
}
fn action(historic: bool, kind: usize, value: Expr, position: Expr) -> StateAction {
    let field = "tree".to_owned();
    let index = 0;
    match (historic, kind) {
        (false, 0) => StateAction::MerkleInsert {
            field,
            index,
            value,
        },
        (true, 0) => StateAction::HistoricMerkleInsert {
            field,
            index,
            value,
        },
        (false, 1) => StateAction::MerkleInsertIndex {
            field,
            index,
            value,
            position,
        },
        (true, 1) => StateAction::HistoricMerkleInsertIndex {
            field,
            index,
            value,
            position,
        },
        (false, 2) => StateAction::MerkleInsertHash {
            field,
            index,
            hash: value,
        },
        (true, 2) => StateAction::HistoricMerkleInsertHash {
            field,
            index,
            hash: value,
        },
        (false, 3) => StateAction::MerkleInsertHashIndex {
            field,
            index,
            hash: value,
            position,
        },
        (true, 3) => StateAction::HistoricMerkleInsertHashIndex {
            field,
            index,
            hash: value,
            position,
        },
        (false, 4) => StateAction::MerkleInsertIndexDefault {
            field,
            index,
            position,
        },
        (true, 4) => StateAction::HistoricMerkleInsertIndexDefault {
            field,
            index,
            position,
        },
        (false, 5) => StateAction::MerkleResetToDefault { field, index },
        (true, 5) => StateAction::HistoricMerkleResetToDefault { field, index },
        (true, 6) => StateAction::HistoricMerkleResetHistory { field, index },
        _ => unreachable!(),
    }
}
fn returned(historic: bool, full: bool, root: Expr) -> StateReturn {
    let field = "tree".to_owned();
    let index = 0;
    match (historic, full) {
        (false, true) => StateReturn::MerkleIsFull { field, index },
        (true, true) => StateReturn::HistoricMerkleIsFull { field, index },
        (false, false) => StateReturn::MerkleCheckRoot { field, index, root },
        (true, false) => StateReturn::HistoricMerkleCheckRoot { field, index, root },
    }
}
fn root_check(historic: bool, root: Expr) -> Expr {
    let field = "tree".to_owned();
    let index = 0;
    if historic {
        Expr::HistoricMerkleCheckRoot {
            field,
            index,
            root: Box::new(root),
        }
    } else {
        Expr::MerkleCheckRoot {
            field,
            index,
            root: Box::new(root),
        }
    }
}
fn checked(c: &Contract) -> Result<CheckedPlan, PlanError> {
    check(
        &c.stateful_circuits[0],
        &c.ledger_fields.iter().map(|f| (f.id.as_str(), f)).collect(),
        &HashMap::new(),
    )
}
fn declarations(c: &Contract) -> declarations::Declarations {
    let ledger = c.ledger_fields.iter().map(|f| (f.id.as_str(), f)).collect();
    let types = TypeEnvironment::from_sources(
        &c.stateful_circuits[0],
        &ledger,
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
    )
    .unwrap();
    declarations::Declarations::new(&ledger, &HashMap::new()).with_type_environment(types)
}
fn candidates(historic: bool) -> Vec<Contract> {
    let mut cases = vec![];
    for kind in 0..if historic { 7 } else { 6 } {
        let mut c = contract(historic, Type::Boolean);
        let value = sample(if kind == 2 || kind == 3 {
            Type::Bytes { length: 32 }
        } else {
            Type::Boolean
        });
        c.stateful_circuits[0]
            .actions
            .push(action(historic, kind, value, sample(uint64())));
        cases.push(c);
    }
    for full in [false, true] {
        let mut c = contract(historic, Type::Boolean);
        c.stateful_circuits[0].result = Type::Boolean;
        c.stateful_circuits[0].return_value = returned(historic, full, sample(digest()));
        cases.push(c);
    }
    cases
}
fn merkle_binding(plan: &Plan) -> usize {
    plan.body
        .bindings
        .iter()
        .position(|b| matches!(b.operation, Operation::Merkle { .. }))
        .unwrap()
}
fn operands(operation: &mut MerkleOperation) -> Vec<&mut Value> {
    match operation {
        MerkleOperation::Insert { value, position } => {
            std::iter::once(value).chain(position.iter_mut()).collect()
        }
        MerkleOperation::InsertHash { hash, position } => {
            std::iter::once(hash).chain(position.iter_mut()).collect()
        }
        MerkleOperation::InsertDefault { position } => vec![position],
        MerkleOperation::CheckRoot { root } => vec![root],
        _ => vec![],
    }
}

#[test]
fn all_seventeen_merkle_source_signatures_render_with_declared_slots() {
    let mut count = 0;
    for historic in [false, true] {
        for c in candidates(historic) {
            let plan = checked(&c).unwrap();
            render(&plan).unwrap();
            crate::render_with_capabilities(&c).unwrap();
            let Operation::Merkle { slot, .. } =
                &plan.plan().body.bindings[merkle_binding(plan.plan())].operation
            else {
                unreachable!()
            };
            assert_eq!((slot.index, slot.depth, slot.historic), (0, 3, historic));
            count += 1;
        }
        let mut c = contract(historic, Type::Boolean);
        c.stateful_circuits[0]
            .actions
            .push(StateAction::Expression {
                value: root_check(historic, sample(digest())),
            });
        render(&checked(&c).unwrap()).unwrap();
    }
    assert_eq!(count, 17);
}

#[test]
fn merkle_leaf_domain_and_nested_physical_paths_are_explicit() {
    for historic in [false, true] {
        for ty in [
            Type::Boolean,
            Type::Field,
            Type::Bytes { length: 32 },
            Type::JubjubPoint,
            Type::OpaqueString,
            Type::OpaqueBytes,
            Type::Struct {
                name: "OwnedLeaf".into(),
                fields: vec![StructField {
                    name: "text".into(),
                    ty: Type::OpaqueString,
                }],
            },
        ] {
            let mut c = contract(historic, ty.clone());
            c.ledger_fields[0].path = vec![0, 1];
            c.stateful_circuits[0]
                .actions
                .push(action(historic, 0, sample(ty), sample(uint64())));
            let plan = checked(&c).unwrap();
            let rendered = render(&plan).unwrap();
            let steps = rendered.steps;
            assert!(
                quote::quote!(#(#steps)*)
                    .to_string()
                    .contains("ledger_slots :: tree")
            );
        }
        for ty in [
            Type::Unit,
            Type::Vector {
                element: Box::new(Type::Boolean),
                length: 1,
            },
            Type::Tuple {
                elements: vec![Type::Boolean],
            },
        ] {
            let mut c = contract(historic, ty);
            c.stateful_circuits[0].result = Type::Boolean;
            c.stateful_circuits[0].return_value = returned(historic, true, sample(digest()));
            assert!(matches!(checked(&c), Err(PlanError::Unsupported(_))));
        }
    }
}

#[test]
fn source_rejects_wrong_leaf_hash_index_root_kind_and_depth() {
    for historic in [false, true] {
        for (kind, value, position) in [
            (0, sample(Type::Field), sample(uint64())),
            (
                1,
                sample(Type::Boolean),
                sample(Type::Unsigned { max: "255".into() }),
            ),
            (2, sample(Type::Bytes { length: 31 }), sample(uint64())),
            (3, sample(Type::Bytes { length: 32 }), sample(Type::Field)),
            (
                4,
                sample(Type::Boolean),
                sample(Type::Unsigned { max: "255".into() }),
            ),
        ] {
            let mut c = contract(historic, Type::Boolean);
            c.stateful_circuits[0]
                .actions
                .push(action(historic, kind, value, position));
            assert!(matches!(checked(&c), Err(PlanError::Unsupported(_))));
        }
        for root in [
            Type::Field,
            Type::Bytes { length: 32 },
            Type::Struct {
                name: "NotMerkleTreeDigest".into(),
                fields: vec![StructField {
                    name: "field".into(),
                    ty: Type::Field,
                }],
            },
        ] {
            let mut c = contract(historic, Type::Boolean);
            c.stateful_circuits[0]
                .actions
                .push(StateAction::Expression {
                    value: root_check(historic, sample(root)),
                });
            assert!(matches!(checked(&c), Err(PlanError::Unsupported(_))));
        }
        for mut c in candidates(historic) {
            c.ledger_fields[0].declaration = declaration(!historic, 3, Type::Boolean);
            assert!(matches!(checked(&c), Err(PlanError::Unsupported(_))));
        }
        for depth in [0, 1, 33, 255] {
            let mut c = contract(historic, Type::Boolean);
            c.ledger_fields[0].declaration = declaration(historic, depth, Type::Boolean);
            c.stateful_circuits[0].actions.push(action(
                historic,
                5,
                sample(Type::Boolean),
                sample(uint64()),
            ));
            assert!(matches!(checked(&c), Err(PlanError::Unsupported(_))));
        }
    }
}
#[test]
fn specialized_root_returns_keep_parameter_scope_while_expression_returns_keep_final_locals() {
    for historic in [false, true] {
        for specialized in [false, true] {
            let mut c = contract(historic, Type::Boolean);
            let root = &mut c.stateful_circuits[0];
            root.result = Type::Boolean;
            root.parameters.push(Parameter {
                name: "root".into(),
                ty: digest(),
            });
            root.actions.push(StateAction::Let {
                bindings: vec![LocalBinding {
                    name: "root".into(),
                    ty: digest(),
                    value: sample(digest()),
                }],
                action: Box::new(StateAction::Sequence { actions: vec![] }),
            });
            let value = Expr::Parameter {
                name: "root".into(),
            };
            root.return_value = if specialized {
                returned(historic, false, value)
            } else {
                StateReturn::Expression {
                    value: root_check(historic, value),
                }
            };
            let checked = checked(&c).unwrap();
            let plan = checked.plan();
            let Operation::Merkle {
                operation: MerkleOperation::CheckRoot { root },
                ..
            } = &plan.body.bindings[merkle_binding(plan)].operation
            else {
                unreachable!()
            };
            let local = plan
                .body
                .bindings
                .iter()
                .find(|b| matches!(b.operation, Operation::Default))
                .unwrap();
            assert_eq!(
                root.id,
                if specialized {
                    plan.parameters[0].id
                } else {
                    local.value.id
                }
            );
            render(&checked).unwrap();
        }
    }
}
