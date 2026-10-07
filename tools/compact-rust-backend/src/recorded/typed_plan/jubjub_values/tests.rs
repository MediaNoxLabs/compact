// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
use serde_json::{Value, json};

fn source() -> Value {
    serde_json::from_str(include_str!(
        "../../../../tests/jubjub-scalar-cell-schema20-ir.json"
    ))
    .unwrap()
}
fn planned(v: &Value) -> Option<TypedPlan> {
    let c: crate::ir::Contract = serde_json::from_value(v.clone()).unwrap();
    lower(
        &c.stateful_circuits[0],
        &c.ledger_fields.iter().map(|f| (f.id.as_str(), f)).collect(),
        &c.witnesses.iter().map(|w| (w.name.as_str(), w)).collect(),
        &c.circuits.iter().map(|p| (p.name.as_str(), p)).collect(),
        &c.stateful_circuits
            .iter()
            .map(|c| (c.name.as_str(), c))
            .collect(),
    )
}
fn first(v: &mut Value, kind: &str, change: &impl Fn(&mut Value)) -> bool {
    if v["kind"] == kind {
        change(v);
        return true;
    }
    match v {
        Value::Array(a) => a.iter_mut().any(|v| first(v, kind, change)),
        Value::Object(m) => m.values_mut().any(|v| first(v, kind, change)),
        _ => false,
    }
}
fn rewrite_names(v: &mut Value) {
    match v {
        Value::String(s) => {
            if ["apply", "point", "scalar", "reduced", "value", "result"].contains(&s.as_str()) {
                *s = format!("renamed_{s}");
            }
        }
        Value::Array(a) => a.iter_mut().for_each(rewrite_names),
        Value::Object(m) => m.values_mut().for_each(rewrite_names),
        _ => {}
    }
}
#[test]
fn nested_terminal_point_has_one_write_and_one_evaluation_per_operation() {
    let p = planned(&source()).unwrap();
    let steps = p.steps;
    let result = p.result;
    let code = quote::quote!(#(#steps)* #result).to_string();
    for op in [
        "jubjub_scalar_from_native",
        "ec_mul_generator",
        "ec_mul",
        "ec_add",
        "record_write",
    ] {
        let needle = format!("{op} (");
        assert_eq!(code.matches(&needle).count(), 1, "{op}: {code}");
    }
    assert!(
        code.find("jubjub_scalar_from_native").unwrap() < code.find("ec_mul_generator").unwrap()
    );
    assert!(code.find("ec_mul_generator (").unwrap() < code.find("ec_mul (").unwrap());
    assert!(code.find("ec_mul (").unwrap() < code.find("ec_add (").unwrap());
    assert!(code.find("ec_add").unwrap() < code.find("record_write").unwrap());
    let c: crate::ir::Contract = serde_json::from_value(source()).unwrap();
    let r = crate::render_with_capabilities(&c).unwrap();
    assert!(r.capabilities.circuits[0].recorded);
    assert!(r.capabilities.circuits[0].observed_call);
}
#[test]
fn names_do_not_admit_the_profile() {
    let mut v = source();
    rewrite_names(&mut v);
    assert!(planned(&v).is_some());
}
#[test]
fn typed_operands_reject_reversed_scalar_and_point() {
    for kind in [
        "jubjub_scalar_from_native",
        "ec_mul_generator",
        "ec_mul",
        "ec_add",
    ] {
        let mut v = source();
        assert!(first(&mut v["stateful_circuits"], kind, &|e| {
            match kind {
                "jubjub_scalar_from_native" => {
                    e["value"] = json!({"kind":"parameter","name":"point"})
                }
                "ec_mul_generator" => e["scalar"] = json!({"kind":"parameter","name":"point"}),
                "ec_mul" => e["point"] = json!({"kind":"parameter","name":"scalar"}),
                "ec_add" => e["right"] = json!({"kind":"parameter","name":"scalar"}),
                _ => unreachable!(),
            }
        }));
        assert!(planned(&v).is_none(), "{kind}");
    }
}
#[test]
fn unknown_or_falsely_annotated_locals_refuse() {
    let mut v = source();
    v["stateful_circuits"][0]["actions"][0]["bindings"][0]["ty"] = json!({"kind":"jubjub_point"});
    assert!(planned(&v).is_none());
    let mut v = source();
    v["stateful_circuits"][0]["actions"][0]["bindings"][0]["value"]["value"]["name"] =
        json!("missing");
    assert!(planned(&v).is_none());
    let mut v = source();
    v["stateful_circuits"][0]["return_value"]["value"]["name"] = json!("missing");
    assert!(planned(&v).is_none());
}
#[test]
fn typed_lexical_shadowing_preserves_inner_return_binding() {
    let mut v = source();
    let a = &mut v["stateful_circuits"][0]["actions"][0];
    a["bindings"][0]["name"] = json!("scalar");
    fn rename(v: &mut Value) {
        match v {
            Value::String(s) if s == "reduced" => *s = "scalar".into(),
            Value::Array(a) => a.iter_mut().for_each(rename),
            Value::Object(m) => m.values_mut().for_each(rename),
            _ => {}
        }
    }
    rename(&mut a["action"]);
    // Alpha-renaming the reduced local to shadow the source parameter must
    // preserve its actual uses. Keeping the outer binding would change output.
    let emitted = |plan: TypedPlan| {
        let steps = plan.steps;
        let result = plan.result;
        quote::quote!(#(#steps)* #result).to_string()
    };
    assert_eq!(
        emitted(planned(&v).unwrap()),
        emitted(planned(&source()).unwrap())
    );
}
#[test]
fn earlier_action_locals_cannot_escape_to_extracted_return() {
    let mut v = source();
    let c = &mut v["stateful_circuits"][0];
    c["actions"]
        .as_array_mut()
        .unwrap()
        .push(json!({"kind":"sequence","actions":[]}));
    assert!(planned(&v).is_none());
}
#[test]
fn declaration_index_path_type_and_duplicate_parameters_refuse() {
    for bad in ["index", "path", "type", "duplicate"] {
        let mut v = source();
        match bad {
            "index" => v["ledger_fields"][0]["index"] = json!(1),
            "path" => v["ledger_fields"][0]["path"] = json!([0, 0]),
            "type" => v["ledger_fields"][0]["declaration"]["ty"] = json!({"kind":"field"}),
            "duplicate" => v["stateful_circuits"][0]["parameters"][1]["name"] = json!("point"),
            _ => unreachable!(),
        };
        assert!(planned(&v).is_none(), "{bad}");
    }
}
#[test]
fn extra_write_or_unrecognized_effect_refuses() {
    for extra in [
        json!({"kind":"cell_write","field":"result","index":0,"value":{"kind":"parameter","name":"point"}}),
        json!({"kind":"expression","value":{"kind":"unit"}}),
    ] {
        let mut v = source();
        v["stateful_circuits"][0]["actions"]
            .as_array_mut()
            .unwrap()
            .insert(0, extra);
        assert!(planned(&v).is_none());
    }
}
#[test]
fn helper_witness_branch_and_other_curve_operations_are_not_fallbacks() {
    for replacement in [
        json!({"kind":"default","ty":{"kind":"jubjub_point"}}),
        json!({"kind":"call","name":"helper","arguments":[]}),
        json!({"kind":"witness_call","name":"w","arguments":[]}),
        json!({"kind":"if","condition":{"kind":"boolean","value":true},"then":{"kind":"parameter","name":"point"},"otherwise":{"kind":"parameter","name":"point"}}),
        json!({"kind":"ec_neg","value":{"kind":"parameter","name":"point"}}),
        json!({"kind":"hash_to_curve","value":{"kind":"parameter","name":"scalar"}}),
    ] {
        let mut v = source();
        first(&mut v["stateful_circuits"], "ec_add", &|x| {
            *x = replacement.clone()
        });
        assert!(planned(&v).is_none());
    }
}
#[test]
fn direct_expression_writes_are_supported_without_source_shape_matching() {
    let mut v = source();
    let c = &mut v["stateful_circuits"][0];
    c["actions"] = json!([{"kind":"cell_write","field":"result","index":0,"value":{"kind":"ec_mul_generator","scalar":{"kind":"jubjub_scalar_from_native","value":{"kind":"parameter","name":"scalar"}}}}]);
    c["return_value"]["value"] = json!({"kind":"parameter","name":"point"});
    assert!(planned(&v).is_some());
}

#[test]
fn nested_jubjub_reduction_and_points_on_default_worker() {
    for depth in [8, 16, 24] {
        let mut v = source();
        let mut scalar = json!({"kind":"parameter","name":"scalar"});
        for _ in 0..depth {
            scalar = json!({"kind":"jubjub_scalar_from_native","value":scalar});
        }
        v["stateful_circuits"][0]["actions"][0]["bindings"][0]["value"] = scalar;
        let contract: crate::ir::Contract = serde_json::from_value(v).unwrap();
        let result = crate::render_with_capabilities(&contract).unwrap();
        assert!(result.capabilities.circuits[0].recorded);

        let mut v = source();
        let mut point = json!({"kind":"parameter","name":"point"});
        for _ in 0..depth {
            point = json!({"kind":"ec_mul","point":point,"scalar":{"kind":"parameter","name":"reduced"}});
        }
        first(&mut v["stateful_circuits"], "ec_add", &|e| {
            *e = point.clone()
        });
        let contract: crate::ir::Contract = serde_json::from_value(v).unwrap();
        let result = crate::render_with_capabilities(&contract).unwrap();
        assert!(result.capabilities.circuits[0].recorded);
    }
}

#[test]
fn point_profile_requires_a_write_and_a_declared_supported_shape() {
    assert!(planned(&source()).is_some());
    for case in [
        "no_write",
        "unit_return",
        "parameter_type",
        "missing_slot",
        "cell_read",
    ] {
        let mut v = source();
        match case {
            "no_write" => {
                v["stateful_circuits"][0]["actions"] = json!([]);
                v["stateful_circuits"][0]["return_value"]["value"] =
                    json!({"kind":"parameter","name":"point"});
            }
            "unit_return" => v["stateful_circuits"][0]["return_value"] = json!({"kind":"unit"}),
            "parameter_type" => {
                v["stateful_circuits"][0]["parameters"][1]["ty"] = json!({"kind":"boolean"})
            }
            "missing_slot" => v["ledger_fields"] = json!([]),
            "cell_read" => assert!(first(
                &mut v["stateful_circuits"][0]["actions"],
                "cell_write",
                &|x| {
                    x["value"] = json!({"kind":"cell_read","field":"result","index":0});
                }
            )),
            _ => unreachable!(),
        }
        assert!(planned(&v).is_none(), "{case}");
    }
}

#[test]
fn expression_local_shadowing_is_scoped_to_the_written_value() {
    let expression = |name: &str| {
        json!({
            "kind":"let",
            "bindings":[{"name":name,"ty":{"kind":"field"},"value":{
                "kind":"jubjub_scalar_from_native","value":{"kind":"parameter","name":"scalar"}
            }}],
            "body":{"kind":"ec_mul_generator","scalar":{"kind":"parameter","name":name}}
        })
    };
    let with_value = |value: Value| {
        let mut v = source();
        v["stateful_circuits"][0]["actions"] = json!([
            {"kind":"cell_write","field":"result","index":0,"value":value}
        ]);
        v["stateful_circuits"][0]["return_value"]["value"] =
            json!({"kind":"parameter","name":"point"});
        v
    };
    let emitted = |plan: TypedPlan| {
        let steps = plan.steps;
        let result = plan.result;
        quote::quote!(#(#steps)* #result).to_string()
    };
    let baseline = with_value(expression("local"));
    assert_eq!(
        emitted(planned(&baseline).unwrap()),
        emitted(planned(&with_value(expression("scalar"))).unwrap())
    );
    let mut escaped = baseline.clone();
    escaped["stateful_circuits"][0]["return_value"]["value"] =
        json!({"kind":"parameter","name":"local"});
    assert!(planned(&escaped).is_none());
    let mut mistyped = baseline;
    mistyped["stateful_circuits"][0]["actions"][0]["value"]["bindings"][0]["ty"] =
        json!({"kind":"jubjub_point"});
    assert!(planned(&mistyped).is_none());
}

#[test]
fn point_result_annotation_does_not_override_the_actual_return_type() {
    let mut v = source();
    assert!(planned(&v).is_some());
    v["stateful_circuits"][0]["return_value"]["value"] =
        json!({"kind":"parameter","name":"scalar"});
    assert!(planned(&v).is_none());
}
