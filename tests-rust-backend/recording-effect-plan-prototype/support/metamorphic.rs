// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Bounded backend-only IR rewrites of the two retained source circuits.
//! These exports have no new Compact source, TypeScript oracle or proof claim.
//! Each stage composes with the previous stage; no effect is moved or duplicated.

use serde_json::{Value, json};

pub fn append_variants(ir: &mut Value) {
    let circuits = ir["stateful_circuits"].as_array_mut().unwrap();
    assert_eq!(circuits.len(), 2);
    let originals = circuits.clone();
    for original in &originals {
        let base = original["name"].as_str().unwrap();
        let mut variant = original.clone();
        alpha_rename(&mut variant);
        append(circuits, &variant, base, "alpha");
        regroup_sequences(&mut variant);
        append(circuits, &variant, base, "sequences");
        shadow_parameters(&mut variant);
        append(circuits, &variant, base, "scopes");
        invert_conditionals(&mut variant);
        append(circuits, &variant, base, "branches");
    }
    let mut empty = originals[0].clone();
    empty["name"] = "empty_public_branch".into();
    let guarded = empty["return_value"]["body"].take();
    empty["return_value"]["body"] = json!({
        "kind":"conditional",
        "condition":{"kind":"equal", "left":{"kind":"parameter", "name":"selector"},
            "right":{"kind":"field_literal", "value":"0"}},
        "then":guarded,
        "otherwise":{"kind":"value", "value":{"kind":"field_literal", "value":"0"}}
    });
    circuits.push(empty);
    assert_eq!(&circuits[..2], originals.as_slice());
}

fn append(circuits: &mut Vec<Value>, variant: &Value, base: &str, suffix: &str) {
    let mut variant = variant.clone();
    variant["name"] = format!("{base}_{suffix}").into();
    circuits.push(variant);
}

// Visit children before rewriting their owner, so inserted syntax is never
// visited again by the same transform.
fn postorder(value: &mut Value, rewrite: &mut impl FnMut(&mut Value)) {
    match value {
        Value::Array(children) => {
            for child in children {
                postorder(child, rewrite);
            }
        }
        Value::Object(fields) => {
            for child in fields.values_mut() {
                postorder(child, rewrite);
            }
        }
        _ => {}
    }
    rewrite(value);
}

fn renamed(name: &Value) -> Value {
    format!("metamorphic_{}", name.as_str().unwrap()).into()
}

fn alpha_rename(circuit: &mut Value) {
    for parameter in circuit["parameters"].as_array_mut().unwrap() {
        parameter["name"] = renamed(&parameter["name"]);
    }
    postorder(&mut circuit["return_value"], &mut |node| {
        if node["kind"] == "parameter" {
            node["name"] = renamed(&node["name"]);
        }
        if let Some(bindings) = node.get_mut("bindings").and_then(Value::as_array_mut) {
            for binding in bindings {
                binding["name"] = renamed(&binding["name"]);
            }
        }
    });
    // Prefixing every local/parameter name is injective and preserves existing
    // shadowing. Ledger names and witness names belong to separate namespaces.
}

fn regroup_sequences(circuit: &mut Value) {
    let mut changed = 0;
    postorder(&mut circuit["return_value"], &mut |node| {
        if node["kind"] != "sequence" || node.get("actions").is_none() {
            return;
        }
        let mut actions = std::mem::take(node["actions"].as_array_mut().unwrap());
        let suffix = actions.split_off(actions.len() / 2);
        node["actions"] = json!([
            {"kind":"sequence", "actions":[]},
            {"kind":"sequence", "actions":[
                {"kind":"sequence", "actions":actions},
                {"kind":"sequence", "actions":suffix}
            ]},
            {"kind":"sequence", "actions":[]}
        ]);
        changed += 1;
    });
    assert!(changed > 0);
}

fn identity_scope(body: Value, name: &str) -> Value {
    json!({
        "kind":"let",
        "bindings":[{"name":name, "ty":{"kind":"field"},
            "value":{"kind":"parameter", "name":name}}],
        "result":body
    })
}

fn shadow_parameters(circuit: &mut Value) {
    let selector = circuit["parameters"][0]["name"]
        .as_str()
        .unwrap()
        .to_owned();
    let next = circuit["parameters"][1]["name"]
        .as_str()
        .unwrap()
        .to_owned();
    let body = &mut circuit["return_value"]["body"];
    postorder(body, &mut |node| {
        if node["kind"] == "conditional" {
            for branch in ["then", "otherwise"] {
                let prior = node[branch].take();
                node[branch] = identity_scope(prior, &selector);
            }
        }
    });
    // The initializer reads the outer binding before the inner same-name
    // binding is introduced. Branch scopes then shadow that binding again.
    *body = identity_scope(identity_scope(body.take(), &selector), &next);
}

fn invert_conditionals(circuit: &mut Value) {
    let mut changed = 0;
    postorder(&mut circuit["return_value"], &mut |node| {
        if node["kind"] == "conditional" {
            let condition = node["condition"].take();
            node["condition"] = json!({"kind":"equal", "left":condition,
                "right":{"kind":"boolean", "value":false}});
            let then = node["then"].take();
            node["then"] = node["otherwise"].take();
            node["otherwise"] = then;
            changed += 1;
        }
    });
    assert_eq!(
        changed, 2,
        "both original nested conditionals must be inverted"
    );
}
