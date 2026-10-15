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

//! tests for recording profile admission and lowering.

use super::*;

#[test]
fn membership_scalar_helpers_keep_counter_provenance_and_declaration_scope() {
    fn planned(value: &serde_json::Value) -> Option<TypedPlan> {
        let c: crate::ir::Contract = serde_json::from_value(value.clone()).unwrap();
        lower(
            &c.stateful_circuits[0],
            &c.ledger_fields.iter().map(|f| (f.id.as_str(), f)).collect(),
            &c.witnesses.iter().map(|w| (w.name.as_str(), w)).collect(),
            &c.circuits.iter().map(|p| (p.name.as_str(), p)).collect(),
            &c.stateful_circuits
                .iter()
                .map(|s| (s.name.as_str(), s))
                .collect(),
        )
    }
    use serde_json::json;
    let source: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/micro-dao-reveal-schema20-ir.json"
    ))
    .unwrap();
    let positive = planned(&source).unwrap();
    let steps = positive.steps;
    let tokens = quote::quote!(#(#steps)*).to_string();
    assert_eq!(tokens.matches("round . record_read").count(), 2);
    assert_eq!(tokens.matches("runtime :: persistent_hash").count(), 2);
    assert_eq!(tokens.matches("pure_circuits :: path_root").count(), 1);
    let renamed = source
        .to_string()
        .replace("reveal_nullifier", "membership_key")
        .replace("commit_with_sk", "membership_commitment");
    assert!(planned(&serde_json::from_str(&renamed).unwrap()).is_some());
    let rejected = |path: &str, replacement: serde_json::Value| {
        let mut changed = source.clone();
        *changed.pointer_mut(path).unwrap() = replacement;
        assert!(planned(&changed).is_none(), "accepted mutation at {path}");
    };
    rejected("/stateful_circuits/1/result", json!({"kind":"boolean"}));
    rejected("/stateful_circuits/1/parameters", json!([]));
    rejected(
        "/stateful_circuits/1/parameters/0/ty",
        json!({"kind":"field"}),
    );
    rejected(
        "/stateful_circuits/1/actions",
        json!([{"kind":"counter_increment","field":"round","index":6,"amount":{"kind":"literal","value":1}}]),
    );
    rejected(
        "/stateful_circuits/1/return_value/value/value/elements/1/value/value/index",
        json!(5),
    );
    rejected(
        "/stateful_circuits/1/return_value/value/value/elements/1/value/value",
        json!({"kind":"witness_call","name":"local_secret_key","arguments":[]}),
    );
    rejected(
        "/stateful_circuits/1/return_value/value/value/elements/2",
        json!({"kind":"parameter","name":"caller_only"}),
    );
    rejected(
        "/stateful_circuits/1/return_value/value/value/elements",
        json!([]),
    );
    rejected(
        "/stateful_circuits/1/return_value/value",
        json!({"kind":"call","name":"reveal_nullifier","arguments":[{"kind":"parameter","name":"sk"}]}),
    );
    let mut ambiguous = source.clone();
    ambiguous["circuits"].as_array_mut().unwrap().push(json!({"name":"reveal_nullifier","parameters":[],"result":{"kind":"bytes","length":32},"body":{"kind":"bytes_literal","bytes":vec![0;32]}}));
    assert!(planned(&ambiguous).is_none());
    let mut open = source.clone();
    open["stateful_circuits"][0]["actions"].as_array_mut().unwrap().insert(0,json!({"kind":"let","bindings":[{"name":"unrelated_read","ty":{"kind":"unsigned","max":"18446744073709551615"},"value":{"kind":"counter_read","field":"round","index":6}}],"action":{"kind":"sequence","actions":[]}}));
    assert!(
        planned(&open).is_none(),
        "root Counter read must not acquire helper provenance"
    );
    let mut declaration = source.clone();
    let slot = declaration["ledger_fields"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|f| f["id"] == "round")
        .unwrap();
    slot["declaration"] =
        json!({"kind":"cell","ty":{"kind":"unsigned","max":"18446744073709551615"}});
    assert!(planned(&declaration).is_none());
    fn mutate_bytes_branch(value: &mut serde_json::Value) -> bool {
        if value["kind"] == "if" && value["then"]["kind"] == "bytes_literal" {
            value["otherwise"] = json!({"kind":"boolean","value":false});
            return true;
        }
        match value {
            serde_json::Value::Object(m) => m.values_mut().any(mutate_bytes_branch),
            serde_json::Value::Array(a) => a.iter_mut().any(mutate_bytes_branch),
            _ => false,
        }
    }
    let mut branch = source.clone();
    assert!(mutate_bytes_branch(&mut branch["stateful_circuits"][0]));
    assert!(planned(&branch).is_none());
}

#[test]
fn context_queries_audit_pure_types_scope_cycles_and_effect_order() {
    fn planned(value: &serde_json::Value) -> Option<TypedPlan> {
        let c: crate::ir::Contract = serde_json::from_value(value.clone()).unwrap();
        lower_context_query(
            &c.stateful_circuits[0],
            &HashMap::new(),
            &c.witnesses.iter().map(|w| (w.name.as_str(), w)).collect(),
            &c.circuits.iter().map(|c| (c.name.as_str(), c)).collect(),
        )
    }
    let source: serde_json::Value =
        serde_json::from_str(include_str!("../../../tests/token-query-schema20-ir.json")).unwrap();
    let positive = planned(&source).unwrap();
    let steps = positive.steps;
    let tokens = quote::quote!(#(#steps)*).to_string();
    assert_eq!(tokens.matches("kernel_self").count(), 1);
    assert_eq!(tokens.matches("persistent_commit").count(), 1);
    assert!(tokens.find("100u8").unwrap() < tokens.find("kernel_self").unwrap());
    assert!(tokens.find("kernel_self").unwrap() < tokens.find("persistent_commit").unwrap());
    // Renaming is metadata, and the original unrelated witnesses stay retained.
    assert!(!source["witnesses"].as_array().unwrap().is_empty());
    let renamed = source
        .to_string()
        .replace("tokenType", "derive_resource")
        .replace("dao_voting_token", "resource_id");
    assert!(planned(&serde_json::from_str(&renamed).unwrap()).is_some());
    let rejected = |path: &str, replacement: serde_json::Value| {
        let mut changed = source.clone();
        *changed.pointer_mut(path).unwrap() = replacement;
        assert!(planned(&changed).is_none(), "accepted mutation at {path}");
    };
    use serde_json::json;
    rejected("/circuits/0/body/opening/bytes", json!([1, 2]));
    rejected("/circuits/0/body/value/elements", json!([]));
    rejected("/circuits/0/parameters/1/name", json!("domain_sep"));
    rejected(
        "/stateful_circuits/0/return_value/value/arguments/1/value/ty",
        json!({"kind":"bytes","length":32}),
    );
    rejected(
        "/circuits/0/body",
        json!({"kind":"create_zswap_output","coin":{"kind":"parameter","name":"domain_sep"},"recipient":{"kind":"parameter","name":"contractAddress"}}),
    );
    rejected("/circuits/0/result", json!({"kind":"boolean"}));
    rejected(
        "/circuits/0/parameters/0/ty",
        json!({"kind":"struct","name":"ContractAddress","fields":[{"name":"bytes","ty":{"kind":"bytes","length":32}}]}),
    );
    rejected("/circuits/0/parameters", json!([]));
    rejected("/circuits/0/body/value/elements/1/field", json!("wrong"));
    rejected("/circuits/0/body/value/elements/1/index", json!(1));
    rejected(
        "/circuits/0/body",
        json!({"kind":"call","name":"tokenType","arguments":[]}),
    );
    rejected(
        "/circuits/0/body",
        json!({"kind":"parameter","name":"caller_only"}),
    );
    rejected(
        "/circuits/0/body",
        json!({"kind":"witness_call","name":"local_secret_key","arguments":[]}),
    );
    rejected(
        "/circuits/0/body",
        json!({"kind":"cell_read","field":"organizer","index":0}),
    );
    rejected(
        "/circuits/0/body",
        source["stateful_circuits"][0]["return_value"]["value"]["arguments"][1]["value"].clone(),
    );
    rejected(
        "/stateful_circuits/0/parameters",
        json!([{"name":"extra","ty":{"kind":"bytes","length":32}}]),
    );
    rejected(
        "/stateful_circuits/0/actions",
        json!([{"kind":"cell_write","field":"organizer","index":0,"value":{"kind":"bytes_literal","bytes":vec![0;32]}}]),
    );
    rejected(
        "/stateful_circuits/0/return_value/value",
        json!({"kind":"bytes_literal","bytes":vec![0;32]}),
    );
    // Caller locals cannot leak into the callee. Both caller arguments still
    // evaluate once in order, including a query whose result is discarded.
    let mut ordered = source.clone();
    let address = source["circuits"][0]["parameters"][1]["ty"].clone();
    let root = &mut ordered["stateful_circuits"][0]["return_value"]["value"];
    let first = root["arguments"][0].clone();
    root["arguments"][0] = json!({"kind":"let","bindings":[{"name":"caller_only","ty":address.clone(),"value":{"kind":"kernel_self","ty":address}}],"body":first});
    let p = planned(&ordered).unwrap();
    let steps = p.steps;
    let tokens = quote::quote!(#(#steps)*).to_string();
    let positions: Vec<_> = tokens
        .match_indices("kernel_self")
        .map(|(n, _)| n)
        .collect();
    assert_eq!(positions.len(), 2);
    assert!(positions[0] < tokens.find("100u8").unwrap());
    assert!(tokens.find("100u8").unwrap() < positions[1]);
    assert!(positions[1] < tokens.find("persistent_commit").unwrap());
    ordered["circuits"][0]["body"] = json!({"kind":"parameter","name":"caller_only"});
    assert!(planned(&ordered).is_none());
}

#[test]
fn composite_helpers_are_typed_scoped_acyclic_and_evaluate_arguments_once() {
    fn planned(contract: &crate::ir::Contract, index: usize) -> Option<TypedPlan> {
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
        let circuits = contract
            .stateful_circuits
            .iter()
            .map(|c| (c.name.as_str(), c))
            .collect();
        lower_composite(
            &contract.stateful_circuits[index],
            &ledger,
            &witnesses,
            &HashMap::new(),
            &circuits,
        )
    }
    let source: crate::ir::Contract = serde_json::from_str(include_str!(
        "../../../tests/stateful-struct-schema20-ir.json"
    ))
    .unwrap();
    for index in 0..3 {
        assert!(planned(&source, index).is_some());
    }
    assert!(planned(&source, 3).is_some());
    let mut extra_effect = source.clone();
    extra_effect.stateful_circuits[0]
        .actions
        .push(StateAction::CellWrite {
            field: "marker".into(),
            index: 0,
            value: Expr::FieldLiteral { value: "1".into() },
        });
    assert!(planned(&extra_effect, 2).is_none());
    let mut cycle = source.clone();
    cycle.stateful_circuits[0].return_value = StateReturn::Expression {
        value: Expr::Call {
            name: "snapshot".into(),
            arguments: vec![Expr::Boolean { value: true }],
        },
    };
    assert!(planned(&cycle, 0).is_none());
    assert!(planned(&cycle, 2).is_none());
    let mut wrong_signature = source.clone();
    wrong_signature.stateful_circuits[0].parameters.clear();
    assert!(planned(&wrong_signature, 2).is_none());
    let mut wrong_result = source.clone();
    wrong_result.stateful_circuits[0].result = contract_address_type();
    assert!(planned(&wrong_result, 2).is_none());
    let mut mismatched_branch = source.clone();
    let StateReturn::Expression {
        value: Expr::If { otherwise, .. },
    } = &mut mismatched_branch.stateful_circuits[0].return_value
    else {
        unreachable!()
    };
    **otherwise = Expr::Default {
        ty: contract_address_type(),
    };
    assert!(planned(&mismatched_branch, 0).is_none());
    let mut leakage = source.clone();
    let caller_value = match leakage.stateful_circuits[2].return_value.clone() {
        StateReturn::Expression { value } => value,
        _ => unreachable!(),
    };
    let snapshot_type = leakage.stateful_circuits[0].result.clone();
    leakage.stateful_circuits[2].return_value = StateReturn::Expression {
        value: Expr::Let {
            bindings: vec![LocalBinding {
                name: "caller_only".into(),
                ty: snapshot_type.clone(),
                value: Expr::Default { ty: snapshot_type },
            }],
            body: Box::new(caller_value),
        },
    };
    leakage.stateful_circuits[0].return_value = StateReturn::Expression {
        value: Expr::Parameter {
            name: "caller_only".into(),
        },
    };
    assert!(planned(&leakage, 2).is_none());

    let mut ordered = source;
    ordered.stateful_circuits[0]
        .parameters
        .push(crate::ir::Parameter {
            name: "unused_second".into(),
            ty: Type::Boolean,
        });
    let StateReturn::Expression {
        value: Expr::StructLiteral { fields, .. },
    } = &mut ordered.stateful_circuits[2].return_value
    else {
        unreachable!()
    };
    let Expr::Call { arguments, .. } = &mut fields[0] else {
        unreachable!()
    };
    *arguments = [71, 72]
        .into_iter()
        .map(|tag| Expr::Let {
            bindings: vec![LocalBinding {
                name: "caller_argument".into(),
                ty: Type::Unsigned {
                    max: u64::MAX.to_string(),
                },
                value: Expr::WitnessCall {
                    name: "next_value".into(),
                    arguments: vec![Expr::UnsignedLiteral {
                        max: "255".into(),
                        value: tag.to_string(),
                    }],
                },
            }],
            body: Box::new(Expr::Boolean { value: true }),
        })
        .collect();
    let plan = planned(&ordered, 2).unwrap();
    let steps = plan.steps;
    let tokens = quote::quote!(#(#steps)*).to_string();
    assert_eq!(tokens.matches("71u128").count(), 1);
    assert_eq!(tokens.matches("72u128").count(), 1);
    assert_eq!(tokens.matches("next_value").count(), 5);
    assert!(tokens.find("71u128").unwrap() < tokens.find("72u128").unwrap());
    assert!(tokens.find("72u128").unwrap() < tokens.find("(1u128)").unwrap());
}

#[test]
fn local_coercion_uses_its_actual_type_before_a_helper_boundary() {
    let ledger = HashMap::new();
    let witnesses = HashMap::new();
    let pure = HashMap::new();
    let mut plan = Plan {
        declarations: Declarations {
            ledger: &ledger,
            witnesses: &witnesses,
            pure: &pure,
            stateful_circuits: None,
        },
        evidence: EffectEvidence::default(),
        admission: AdmissionPolicy {
            field_cell_slot: None,
            effectful_field_cells: false,
            read_only_assertions: false,
            unit_actions: false,
            phase_reset: false,
            composite_domain: CompositeDomain::None,
            composition_calls: None,
            composition_lookup_sites: HashSet::new(),
            counter_hash_helpers: false,
        },
        evaluation: EvaluationState {
            next: 0,
            context_query: false,
            scalar_arguments: false,
            scalar_body_depth: 0,
            active_calls: HashSet::new(),
        },
    };
    let actual = Type::Unsigned { max: "255".into() };
    let target = Type::Unsigned {
        max: "65535".into(),
    };
    let scope = HashMap::from([(
        "n".into(),
        TypedValue {
            ty: actual,
            value: syn::parse_quote!(small),
        },
    )]);
    let expression = Expr::Coerce {
        value: Box::new(Expr::Parameter { name: "n".into() }),
        ty: target.clone(),
    };
    let mut steps = Vec::new();
    let value = plan.expression(&expression, &scope, &mut steps).unwrap();
    assert_eq!(value.ty, target);
    let item: syn::Item = syn::parse_quote!(fn probe(small:runtime::BoundedUint<255>)->Result<(),runtime::CompactError>{#(#steps)* Ok(())});
    let file = syn::File {
        shebang: None,
        attrs: vec![],
        items: vec![item],
    };
    let text = prettyplease::unparse(&file);
    assert!(
        text.split_whitespace()
            .collect::<String>()
            .contains("cast_unsigned::<255,65535,>(small)?"),
        "{text}"
    );
    let narrowing = Expr::Coerce {
        value: Box::new(Expr::Parameter { name: "n".into() }),
        ty: Type::Unsigned { max: "15".into() },
    };
    assert!(
        plan.expression(&narrowing, &scope, &mut Vec::new())
            .is_none()
    );
}

#[test]
fn counter_parameters_require_the_actual_u16_bound() {
    let source = include_str!("../../../tests/election-schema13-ir.json");
    let contract: crate::ir::Contract = serde_json::from_str(source).unwrap();
    let ledger = contract
        .ledger_fields
        .iter()
        .map(|field| (field.id.as_str(), field))
        .collect();
    let witnesses = HashMap::new();
    let pure = HashMap::new();
    let mut plan = Plan {
        declarations: Declarations {
            ledger: &ledger,
            witnesses: &witnesses,
            pure: &pure,
            stateful_circuits: None,
        },
        evidence: EffectEvidence::default(),
        admission: AdmissionPolicy {
            field_cell_slot: None,
            effectful_field_cells: false,
            read_only_assertions: false,
            unit_actions: false,
            phase_reset: false,
            composite_domain: CompositeDomain::None,
            composition_calls: None,
            composition_lookup_sites: HashSet::new(),
            counter_hash_helpers: false,
        },
        evaluation: EvaluationState {
            next: 0,
            context_query: false,
            scalar_arguments: false,
            scalar_body_depth: 0,
            active_calls: HashSet::new(),
        },
    };
    let action = StateAction::CounterIncrement {
        field: "tally_yes".into(),
        index: 3,
        amount: CounterAmount::Parameter {
            name: "amount".into(),
        },
    };
    for (max, expected) in [("65535", true), ("255", false), ("65536", false)] {
        let scope = HashMap::from([(
            "amount".into(),
            TypedValue {
                ty: Type::Unsigned { max: max.into() },
                value: syn::parse_quote!(amount),
            },
        )]);
        assert_eq!(
            plan.action(&action, &scope, &mut Vec::new()).is_some(),
            expected
        );
    }
}

#[test]
fn witness_eligibility_is_local_but_counts_all_audited_branches() {
    fn admitted(contract: &crate::ir::Contract, index: usize) -> bool {
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
        lower(
            &contract.stateful_circuits[index],
            &ledger,
            &witnesses,
            &HashMap::new(),
            &HashMap::new(),
        )
        .is_some()
    }
    let mut field: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/root-let-action-return-schema17-ir.json"
    ))
    .unwrap();
    field["witnesses"] =
        serde_json::json!([{"name":"unrelated", "parameters":[], "result":{"kind":"field"}}]);
    assert!(admitted(&serde_json::from_value(field.clone()).unwrap(), 0));
    field["stateful_circuits"][0]["actions"][0]["action"]["actions"][0]["bindings"][0]["value"]["right"] =
        serde_json::json!({"kind":"witness_call", "name":"unrelated", "arguments":[]});
    let mut field: crate::ir::Contract = serde_json::from_value(field).unwrap();
    field.schema_version = crate::ir::SCHEMA_VERSION;
    assert!(!admitted(&field, 0));
    // The legacy profile still rejects witnesses; checked composition
    // admits this fully typed zero-argument Field witness.
    let rendered = crate::render_with_capabilities(&field).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    let source: String = rendered
        .source
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    assert!(source.contains("frame.try_witness_metered"));
    assert!(source.contains("witnesses.unrelated("));

    let mut counters: crate::ir::Contract = serde_json::from_str(include_str!(
        "../../../tests/counter-less-than-schema15-ir.json"
    ))
    .unwrap();
    counters.witnesses.push(WitnessDeclaration {
        source: None,
        name: "unused_bool".into(),
        parameters: vec![],
        result: Type::Boolean,
    });
    for index in 0..counters.stateful_circuits.len() {
        assert!(admitted(&counters, index));
    }
    // A runtime-false branch is still part of the audited circuit domain.
    let StateReturn::Expression { value } = &mut counters.stateful_circuits[2].return_value else {
        unreachable!()
    };
    let Expr::If {
        condition,
        otherwise,
        ..
    } = value
    else {
        unreachable!()
    };
    **condition = Expr::Boolean { value: true };
    **otherwise = Expr::WitnessCall {
        name: "unused_bool".into(),
        arguments: vec![],
    };
    assert!(!admitted(&counters, 2));
    counters.witnesses.push(WitnessDeclaration {
        source: None,
        name: "used_threshold".into(),
        parameters: vec![],
        result: Type::Unsigned {
            max: u64::MAX.to_string(),
        },
    });
    let StateReturn::Expression {
        value: Expr::CounterLessThan { threshold, .. },
    } = &mut counters.stateful_circuits[0].return_value
    else {
        unreachable!()
    };
    **threshold = Expr::WitnessCall {
        name: "used_threshold".into(),
        arguments: vec![],
    };
    assert!(!admitted(&counters, 0));
    counters.schema_version = crate::ir::SCHEMA_VERSION;
    let capabilities = crate::render_with_capabilities(&counters)
        .unwrap()
        .capabilities;
    // The frozen oracle keeps its effect-count restrictions. Current composition
    // validates the declared witness in both selected and unselected branches.
    assert!(capabilities.circuits[0].recorded && capabilities.circuits[0].observed_call);
    assert!(capabilities.circuits[2].recorded && capabilities.circuits[2].observed_call);
}

#[test]
fn field_cell_root_let_keeps_typed_scope_and_rejects_extra_effects() {
    fn admitted(value: &serde_json::Value) -> bool {
        let contract: crate::ir::Contract = serde_json::from_value(value.clone()).unwrap();
        let ledger = contract
            .ledger_fields
            .iter()
            .map(|field| (field.id.as_str(), field))
            .collect();
        let witnesses = contract
            .witnesses
            .iter()
            .map(|witness| (witness.name.as_str(), witness))
            .collect();
        let pure = HashMap::new();
        lower(
            &contract.stateful_circuits[0],
            &ledger,
            &witnesses,
            &pure,
            &HashMap::new(),
        )
        .is_some()
    }
    let source: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/root-let-action-return-schema17-ir.json"
    ))
    .unwrap();
    assert!(admitted(&source));

    let mut wrong_type = source.clone();
    wrong_type["stateful_circuits"][0]["actions"][0]["bindings"][0]["ty"]["kind"] =
        "boolean".into();
    assert!(!admitted(&wrong_type));

    let mut wrong_index = source.clone();
    wrong_index["stateful_circuits"][0]["actions"][0]["bindings"][0]["value"]["index"] = 1.into();
    assert!(!admitted(&wrong_index));

    let mut wrong_return = source.clone();
    wrong_return["stateful_circuits"][0]["return_value"]["value"]["name"] = "before".into();
    assert!(!admitted(&wrong_return));

    let mut shadowed_return = source.clone();
    shadowed_return["stateful_circuits"][0]["actions"][0]["bindings"][0]["name"] = "echo".into();
    assert!(!admitted(&shadowed_return));

    let mut extra_read = source.clone();
    extra_read["stateful_circuits"][0]["actions"][0]["action"]["actions"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({"kind":"let", "bindings":[{
            "name":"other", "ty":{"kind":"field"},
            "value":{"kind":"cell_read", "field":"stored", "index":0}
        }], "action":{"kind":"sequence", "actions":[]}}));
    assert!(!admitted(&extra_read));

    let mut leaked_local = source.clone();
    leaked_local["stateful_circuits"][0]["actions"][0]["action"]["actions"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({"kind":"cell_write", "field":"stored",
            "index":0, "value":{"kind":"parameter", "name":"after"}}));
    assert!(!admitted(&leaked_local));

    let mut wrong_write_slot = source;
    wrong_write_slot["stateful_circuits"][0]["actions"][0]["action"]["actions"][0]["action"]["actions"]
        [0]["index"] = 1.into();
    assert!(!admitted(&wrong_write_slot));
}
