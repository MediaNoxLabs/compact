// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn canonical_point_products_and_unused_public_parameters_keep_complete_old_admission() {
    for multiple in [false, true] {
        let mut v = reduced();
        edit(&mut v, &mut |m| {
            if m.get("kind") == Some(&json!("struct")) && m.get("name") == Some(&json!("Method")) {
                let fields = m.get_mut("fields").unwrap().as_array_mut().unwrap();
                if multiple {
                    fields.push(json!({"name":"secondPoint","ty":{"kind":"jubjub_point"}}));
                } else {
                    fields.remove(1);
                }
            }
            if m.get("kind") == Some(&json!("struct_literal"))
                && m.get("ty").is_some_and(|t| t["name"] == "Method")
            {
                let fields = m.get_mut("fields").unwrap().as_array_mut().unwrap();
                if multiple {
                    fields.push(json!({"kind":"parameter","name":"initialKey"}));
                } else {
                    fields.remove(1);
                }
            }
        });
        if multiple {
            circuit(&mut v, "verifyLocal")["parameters"]
                .as_array_mut()
                .unwrap()
                .push(json!({"name":"anotherPoint","ty":{"kind":"jubjub_point"}}));
            call(&mut v)["arguments"].as_array_mut().unwrap().push(json!({"kind":"coerce","ty":{"kind":"jubjub_point"},"value":{"kind":"struct_field","value":{"kind":"parameter","name":"method"},"field":"secondPoint","index":2}}));
        }
        // Repeating a root argument is permitted independently of requiring at
        // least one lookup-derived point; the additional formal is intentionally unused.
        circuit(&mut v, "verifyLocal")["parameters"]
            .as_array_mut()
            .unwrap()
            .push(json!({"name":"expectedAgain","ty":{"kind":"jubjub_point"}}));
        call(&mut v)["arguments"].as_array_mut().unwrap().push(
            json!({"kind":"coerce","ty":{"kind":"jubjub_point"},"value":{"kind":"parameter","name":"expected"}}));
        included(&parse(v), "verify");
    }
    for ty in [
        json!({"kind":"unit"}),
        json!({"kind":"opaque_bytes"}),
        json!({"kind":"tuple","elements":vec![json!({"kind":"field"});11]}),
    ] {
        let mut v = reduced();
        let parameters = root(&mut v)["parameters"].as_array_mut().unwrap();
        parameters.swap(0, 1);
        parameters.push(json!({"name":"unused","ty":ty}));
        included(&parse(v), "verify");
    }
}

#[test]
fn transitive_private_helper_branches_and_known_source_exclusion_remain_distinct() {
    let mut v = reduced();
    let mut leaf = circuit(&mut v, "verifyLocal").clone();
    leaf["name"] = json!("privateLeaf");
    v["stateful_circuits"].as_array_mut().unwrap().push(leaf);
    circuit(&mut v, "verifyLocal")["actions"] = json!([{"kind":"if","condition":{"kind":"boolean","value":true},"then":{"kind":"circuit_call","name":"privateLeaf","arguments":[{"kind":"parameter","name":"expected"},{"kind":"parameter","name":"actual"}]},"otherwise":{"kind":"sequence","actions":[]}}]);
    included(&parse(v.clone()), "verify");
    circuit(&mut v, "verifyLocal")["actions"][0]["otherwise"] =
        json!({"kind":"circuit_call","name":"missingLeaf","arguments":[]});
    let c = parse(v);
    assert!(old(&c, "verify").unwrap().is_none());
    assert!(crate::render_with_capabilities(&c).is_err());
    assert!(checked(&c, "verify").is_err());
    let mut wrong_witness = reduced();
    wrong_witness["witnesses"][0]["parameters"] =
        json!([{"name":"required","ty":{"kind":"field"}}]);
    let c = parse(wrong_witness);
    assert!(old(&c, "verify").unwrap().is_none());
    assert!(crate::render_with_capabilities(&c).is_err());
    let mut no_op = reduced();
    circuit(&mut no_op, "verifyLocal")["actions"] = json!([]);
    included(&parse(no_op), "verify");
    // Existing source-policy exception, not a Compact source-language positive:
    // native Rust accepts opaque hashing; the frontend excludes persistent opaque.
    let mut excluded = reduced();
    circuit(&mut excluded, "verifyLocal")["actions"] = json!([local_binding(
        json!({"kind":"persistent_hash","value":{"kind":"default","ty":{"kind":"opaque_string"}}}),
        json!({"kind":"bytes","length":32})
    )]);
    let c = parse(excluded);
    assert!(matches!(
        old(&c, "verify").unwrap(),
        Some(RecordingOutcome::Supported(_))
    ));
    crate::render_with_capabilities(&c).unwrap();
    let Err(effect_plan::PlanError::Unsupported(gap)) = checked(&c, "verify") else {
        panic!("source-excluded persistent opaque policy");
    };
    assert!(gap.detail.contains("persistent hash"));
}

#[test]
fn old_nonapplicability_checked_expansion_and_true_refusal_preserve_native_priority() {
    let mut effectful = reduced();
    circuit(&mut effectful,"verifyLocal")["actions"].as_array_mut().unwrap().push(json!({"kind":"assert","condition":{"kind":"cell_read","field":"active","index":0},"message":"extra helper read"}));
    let c = parse(effectful);
    assert!(old(&c, "verify").unwrap().is_none());
    assert_eq!(
        methods(&current(&c, "verify"))
            .iter()
            .filter(|n| n.as_str() == "record_read")
            .count(),
        2
    );
    let mut fold = reduced();
    circuit(&mut fold, "verifyLocal")["actions"] = json!([local_binding(
        serde_json::to_value(test_support::unsupported_fixed_input()).unwrap(),
        json!({"kind":"field"})
    )]);
    let c = test_support::with_unsupported_fold(parse(fold));
    assert!(old(&c, "verify").unwrap().is_none());
    let output = crate::render_with_capabilities(&c).unwrap();
    assert!(
        !output
            .capabilities
            .circuits
            .iter()
            .find(|r| r.name == "verify")
            .unwrap()
            .recorded
    );
    let Err(effect_plan::PlanError::Unsupported(gap)) = checked(&c, "verify") else {
        panic!("direct fold stays unsupported");
    };
    assert_eq!(gap.ir_node, "Expr::VectorFoldCall");
    assert!(
        gap.path.ends_with("actions[0].bindings[0].value"),
        "{}",
        gap.path
    );
    let mut recursive = reduced();
    circuit(&mut recursive, "verifyLocal")["actions"] = json!([{"kind":"circuit_call","name":"verifyLocal","arguments":[{"kind":"parameter","name":"expected"},{"kind":"parameter","name":"actual"}]}]);
    let c = parse(recursive);
    assert!(old(&c, "verify").unwrap().is_none());
    assert!(crate::render_with_capabilities(&c).is_err());
    assert!(checked(&c, "verify").is_err());
    let mut internal = reduced();
    root(&mut internal)["internal"] = json!(true);
    let c = parse(internal);
    assert!(matches!(
        old(&c, "verify").unwrap(),
        Some(RecordingOutcome::Supported(_))
    ));
    let output = crate::render_with_capabilities(&c).unwrap();
    assert!(
        !output
            .capabilities
            .circuits
            .iter()
            .any(|r| r.name == "verify")
    );
    let mut invalid = reduced();
    root(&mut invalid)["parameters"][1]["name"] = json!("id");
    let c = parse(invalid);
    assert!(old(&c, "verify").unwrap().is_none());
    assert!(crate::render_with_capabilities(&c).is_err());
    // No limits change. Existing calibrated ordinary/recursive worker gates are
    // reused externally rather than multiplying another synthetic depth matrix.
}
