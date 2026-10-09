// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Finite IR-only inclusion probes. Every positive first passes the complete
//! old VotingCommit planner, including its closure typing and effect counts.
//! Isolated helper support does not establish whole voting-family admission.
use super::*;
use serde_json::Value as Json;

fn source() -> Contract {
    serde_json::from_str(include_str!(
        "../../../../tests/micro-dao-vote-commit-schema20-ir.json"
    ))
    .unwrap()
}

struct Environment<'a> {
    ledger: HashMap<&'a str, &'a LedgerField>,
    witnesses: HashMap<&'a str, &'a WitnessDeclaration>,
    pure: HashMap<&'a str, &'a PureCircuit>,
    stateful: HashMap<&'a str, &'a StatefulCircuit>,
}
impl<'a> Environment<'a> {
    fn new(contract: &'a Contract) -> Self {
        Self {
            ledger: contract
                .ledger_fields
                .iter()
                .map(|v| (v.id.as_str(), v))
                .collect(),
            witnesses: contract
                .witnesses
                .iter()
                .map(|v| (v.name.as_str(), v))
                .collect(),
            pure: contract
                .circuits
                .iter()
                .map(|v| (v.name.as_str(), v))
                .collect(),
            stateful: contract
                .stateful_circuits
                .iter()
                .map(|v| (v.name.as_str(), v))
                .collect(),
        }
    }
    fn old_first(&self, label: &str) {
        assert!(
            crate::recorded::typed_plan::lower_voting_commit(
                self.stateful["vote_commit"],
                &self.ledger,
                &self.witnesses,
                &self.pure,
                &self.stateful,
            )
            .is_some(),
            "{label}: complete old VotingCommit planner rejected the probe"
        );
    }
    fn checked(&self, name: &str) -> Result<CheckedPlan, PlanError> {
        check_with_calls(
            self.stateful[name],
            &self.ledger,
            &self.witnesses,
            &self.pure,
            &self.stateful,
        )
    }
}

fn helper_included(contract: &Contract, name: &str, label: &str) {
    let env = Environment::new(contract);
    env.old_first(label);
    let checked = env
        .checked(name)
        .unwrap_or_else(|e| panic!("{label}: old-accepted helper refused: {e:?}"));
    render(&checked).unwrap_or_else(|e| panic!("{label}: checked render failed: {e:?}"));
}

fn bytes(value: u8) -> Expr {
    Expr::BytesLiteral {
        bytes: vec![value; 32],
    }
}
fn parameter(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}

// This is a bounded test-fixture rewrite, not an alternative compiler visitor.
// It changes only call targets/actuals; serde subsequently checks the IR shape.
fn replace_calls(value: &mut Json, old: &str, new: &str, arity: usize, reverse: bool) {
    match value {
        Json::Object(fields) => {
            if fields.get("kind").and_then(Json::as_str) == Some("call")
                && fields.get("name").and_then(Json::as_str) == Some(old)
            {
                fields.insert("name".into(), Json::String(new.into()));
                let arguments = fields.get_mut("arguments").unwrap().as_array_mut().unwrap();
                arguments.truncate(arity);
                while arguments.len() < arity {
                    arguments.push(serde_json::to_value(bytes(23)).unwrap());
                }
                if reverse {
                    arguments.reverse();
                }
            }
            for child in fields.values_mut() {
                replace_calls(child, old, new, arity, reverse);
            }
        }
        Json::Array(children) => {
            for child in children {
                replace_calls(child, old, new, arity, reverse);
            }
        }
        _ => {}
    }
}

fn variant(
    original: &str,
    arity: usize,
    counter_position: usize,
    reverse: bool,
    literal: bool,
) -> (Contract, String) {
    let mut contract = source();
    let name = format!("hash_helper_{arity}_{counter_position}_{reverse}_{literal}");
    let helper = contract
        .stateful_circuits
        .iter_mut()
        .find(|v| v.name == original)
        .unwrap();
    helper.name.clone_from(&name);
    let mut names = vec!["renamed_seed", "renamed_ballot"];
    names.truncate(arity);
    if reverse {
        names.reverse();
    }
    helper.parameters = names
        .iter()
        .map(|name| Parameter {
            name: (*name).into(),
            ty: Type::Bytes { length: 32 },
        })
        .collect();
    let mut elements = vec![
        parameter(names[0]),
        if literal {
            bytes(71)
        } else {
            parameter(names[arity - 1])
        },
    ];
    elements.insert(
        counter_position,
        Expr::FieldToBytes32 {
            value: Box::new(Expr::FieldCast {
                value: Box::new(Expr::CounterRead {
                    field: "round".into(),
                    index: 6,
                }),
            }),
        },
    );
    helper.return_value = StateReturn::Expression {
        value: Expr::PersistentHash {
            value: Box::new(Expr::Tuple { elements }),
        },
    };
    let mut json = serde_json::to_value(contract).unwrap();
    replace_calls(&mut json, original, &name, arity, reverse);
    (serde_json::from_value(json).unwrap(), name)
}

#[test]
fn both_retained_counter_hash_helpers_are_old_first_checked_and_rendered() {
    let contract = source();
    for name in ["commitment_nullifier", "commit_with_sk"] {
        helper_included(&contract, name, name);
    }
}

#[test]
fn finite_counter_hash_grammar_variants_pass_complete_old_plan_before_inclusion() {
    let mut accepted = 0;
    for original in ["commitment_nullifier", "commit_with_sk"] {
        for arity in 1..=2 {
            for counter_position in 0..3 {
                for reverse in [false, true] {
                    for literal in [false, true] {
                        let (contract, name) =
                            variant(original, arity, counter_position, reverse, literal);
                        helper_included(&contract, &name, &format!("{original}/{name}"));
                        accepted += 1;
                    }
                }
            }
        }
    }
    assert_eq!(accepted, 48); // Finite generated cases, not language coverage.
}

#[test]
fn whole_old_voting_plan_progresses_to_transient_upgrade() {
    let contract = source();
    let env = Environment::new(&contract);
    env.old_first("retained whole voting root");
    let PlanError::Unsupported(gap) = env.checked("vote_commit").unwrap_err() else {
        panic!("expected an unsupported source operation, not a checked-plan invariant failure")
    };
    eprintln!("old-first voting gap: {gap:?}");
    assert_eq!(gap.ir_node, "Expr::UpgradeFromTransient");
}

fn phase_assertion(contract: &mut Contract) -> &mut Expr {
    let root = contract
        .stateful_circuits
        .iter_mut()
        .find(|v| v.name == "vote_commit")
        .unwrap();
    let StateAction::Sequence { actions } = &mut root.actions[0] else {
        panic!("source sequence")
    };
    let StateAction::Assert { condition, .. } = &mut actions[0] else {
        panic!("source phase assertion")
    };
    condition
}

#[test]
fn old_first_optional_string_cell_observations_progress_to_transient_upgrade() {
    // Three finite Boolean contexts preserve one Cell read and the original
    // local_state witness. Each complete old planner acceptance is mandatory.
    for mode in ["presence", "identity", "equal_false"] {
        let mut contract = source();
        let topic = contract
            .ledger_fields
            .iter()
            .find(|v| v.id == "topic")
            .unwrap();
        let LedgerFieldKind::Cell {
            ty: Type::Struct { fields, .. },
        } = &topic.declaration
        else {
            panic!("retained topic must be a named optional-string Cell")
        };
        assert_eq!(fields[0].ty, Type::Boolean);
        assert_eq!(fields[1].ty, Type::OpaqueString);
        let projection = Expr::StructField {
            value: Box::new(Expr::CellRead {
                field: topic.id.clone(),
                index: topic.index,
            }),
            field: fields[0].name.clone(),
            index: 0,
        };
        let observation = match mode {
            "presence" => projection,
            "identity" => Expr::Coerce {
                value: Box::new(projection),
                ty: Type::Boolean,
            },
            "equal_false" => Expr::Equal {
                left: Box::new(projection),
                right: Box::new(Expr::Boolean { value: false }),
            },
            _ => unreachable!(),
        };
        let Expr::If { condition, .. } = phase_assertion(&mut contract) else {
            panic!("source conditional phase assertion")
        };
        **condition = observation;
        let env = Environment::new(&contract);
        env.old_first(mode);
        let PlanError::Unsupported(gap) = env.checked("vote_commit").unwrap_err() else {
            panic!("expected a remaining unsupported source operation")
        };
        eprintln!("old-first optional-string {mode} next gap: {gap:?}");
        assert_eq!(gap.ir_node, "Expr::UpgradeFromTransient", "{mode}: {gap:?}");
    }
}

#[test]
fn old_first_opaque_bytes_cell_does_not_admit_an_opaque_witness_interface() {
    let mut contract = source();
    let topic = contract
        .ledger_fields
        .iter_mut()
        .find(|v| v.id == "topic")
        .unwrap();
    topic.declaration = LedgerFieldKind::Cell {
        ty: Type::OpaqueBytes,
    };
    let read = Expr::CellRead {
        field: topic.id.clone(),
        index: topic.index,
    };
    let witness = contract
        .witnesses
        .iter_mut()
        .find(|v| v.name == "local_state")
        .unwrap();
    assert!(witness.parameters.is_empty());
    witness.result = Type::OpaqueBytes;
    // Replace the whole assertion condition: one Cell read and one zero-arg
    // witness remain, retaining every legacy root effect-count obligation.
    *phase_assertion(&mut contract) = Expr::Equal {
        left: Box::new(read.clone()),
        right: Box::new(Expr::WitnessCall {
            name: "local_state".into(),
            arguments: vec![],
        }),
    };
    let env = Environment::new(&contract);
    env.old_first("opaque bytes Cell compared with declared opaque witness");

    // Isolate the exact same read from that old-accepted full root. Its value
    // stays local; neither the isolated root result nor its ABI is opaque.
    let isolated = StatefulCircuit {
        source: None,
        name: "owned_read_probe".into(),
        internal: false,
        parameters: vec![],
        result: Type::Unit,
        return_value: StateReturn::Unit,
        actions: vec![StateAction::Let {
            bindings: vec![LocalBinding {
                name: "observed".into(),
                ty: Type::OpaqueBytes,
                value: read,
            }],
            action: Box::new(StateAction::Sequence { actions: vec![] }),
        }],
    };
    let checked = check_with_calls(
        &isolated,
        &env.ledger,
        &env.witnesses,
        &env.pure,
        &env.stateful,
    )
    .expect("old-accepted OpaqueBytes Cell read must be a checked local value");
    render(&checked).unwrap();
    let PlanError::Unsupported(gap) = env.checked("vote_commit").unwrap_err() else {
        panic!("opaque witness interface must retain its independent refusal")
    };
    eprintln!("old-first opaque bytes next gap: {gap:?}");
    assert_eq!(gap.ir_node, "RecordedPlan");
    assert_eq!(
        gap.detail,
        "witness result must be supported and arguments must match arity"
    );
    assert!(gap.path.ends_with(".right"), "{gap:?}");
}

#[test]
fn whole_old_voting_accepts_wrapper_and_qualified_membership_before_isolated_inclusion() {
    // IR-only variations of the whole retained voting root. Mandatory ordinary
    // participant insertion remains Bytes32; only the phase observation varies.
    for ty in [
        Type::Struct {
            name: "ParticipantKey".into(),
            fields: vec![StructField {
                name: "digest".into(),
                ty: Type::Bytes { length: 32 },
            }],
        },
        crate::coin_shapes::qualified_coin_type(),
    ] {
        let mut contract = source();
        let state = contract
            .ledger_fields
            .iter_mut()
            .find(|v| v.id == "state")
            .unwrap();
        state.declaration = LedgerFieldKind::Cell { ty: Type::Boolean };
        let state_index = state.index;
        contract
            .witnesses
            .iter_mut()
            .find(|v| v.name == "local_state")
            .unwrap()
            .result = ty.clone();
        let index = u8::try_from(contract.ledger_fields.len()).unwrap();
        contract.ledger_fields.push(LedgerField {
            id: "observed_members".into(),
            index,
            path: vec![index],
            source: None,
            declaration: LedgerFieldKind::Set { ty: ty.clone() },
        });
        let observation = Expr::SetMember {
            field: "observed_members".into(),
            index,
            value: Box::new(Expr::WitnessCall {
                name: "local_state".into(),
                arguments: vec![],
            }),
        };
        *phase_assertion(&mut contract) = Expr::If {
            condition: Box::new(observation.clone()),
            then: Box::new(Expr::CellRead {
                field: "state".into(),
                index: state_index,
            }),
            otherwise: Box::new(Expr::Boolean { value: false }),
        };
        let env = Environment::new(&contract);
        env.old_first(&format!("additional membership {ty:?}"));
        let isolated = StatefulCircuit {
            name: "membership_probe".into(),
            source: None,
            internal: false,
            parameters: vec![],
            result: Type::Boolean,
            actions: vec![],
            return_value: StateReturn::Expression { value: observation },
        };
        let checked = check_with_calls(
            &isolated,
            &env.ledger,
            &env.witnesses,
            &env.pure,
            &env.stateful,
        )
        .expect("whole-old-accepted membership must lower independently");
        render(&checked).unwrap();
        let PlanError::Unsupported(gap) = env.checked("vote_commit").unwrap_err() else {
            panic!("remaining whole voting operation must be a source refusal")
        };
        eprintln!("old-first membership {ty:?}, next whole-root gap: {gap:?}");
        assert_eq!(gap.ir_node, "Expr::UpgradeFromTransient", "{gap:?}");
    }
}
