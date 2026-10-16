// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn original_flat_and_chunked_contracts_keep_reads_before_private_helper() {
    let full: Contract =
        serde_json::from_str(include_str!("../../../tests/did-digest-schema20-ir.json")).unwrap();
    for (c, name) in [
        (full, "verifySchnorrJubjubDigestSignature"),
        (parse(reduced()), "verify"),
    ] {
        let names = methods(&included(&c, name));
        let positions: Vec<_> = [
            "record_read",
            "record_member",
            "record_lookup",
            "try_witness_metered",
        ]
        .into_iter()
        .map(|n| names.iter().position(|x| x == n).unwrap())
        .collect();
        assert!(positions.windows(2).all(|w| w[0] < w[1]));
        assert!(!names.iter().any(|n| n == "call_local"));
    }
}

#[test]
fn physical_and_declaration_identity_outlive_the_old_same_map_relation() {
    let mut v = reduced();
    edit(&mut v, &mut |m| {
        if let Some(index) = m.get("index").and_then(Value::as_u64)
            && (m.contains_key("declaration")
                || m.get("field")
                    .is_some_and(|v| matches!(v.as_str(), Some("active" | "methods"))))
        {
            m.insert("index".into(), json!(1 - index));
            if m.contains_key("path") {
                m.insert("path".into(), json!([1 - index]));
            }
        }
    });
    rename(&mut v, "active", "r#type");
    rename(&mut v, "methods", "r#match");
    rename(&mut v, "verifyLocal", "local$verifier");
    included(&parse(v), "verify");
    let mut different = reduced();
    let mut other = different["ledger_fields"][1].clone();
    other["id"] = json!("otherMethods");
    other["index"] = json!(2);
    other["path"] = json!([2]);
    different["ledger_fields"]
        .as_array_mut()
        .unwrap()
        .push(other);
    lookup_let(&mut different)["bindings"][0]["value"]["field"] = json!("otherMethods");
    lookup_let(&mut different)["bindings"][0]["value"]["index"] = json!(2);
    let c = parse(different);
    assert!(old(&c, "verify").unwrap().is_none());
    current(&c, "verify"); // Native-valid two-Map composition is an expansion.
    for invalid in ["index", "kind"] {
        let mut v = reduced();
        if invalid == "index" {
            lookup_let(&mut v)["bindings"][0]["value"]["index"] = json!(0);
        } else {
            v["ledger_fields"][1]["declaration"] = json!({"kind":"cell","ty":{"kind":"boolean"}});
        }
        let c = parse(v);
        assert!(old(&c, "verify").unwrap().is_none());
        assert!(crate::render_with_capabilities(&c).is_err(), "{invalid}");
        assert!(checked(&c, "verify").is_err(), "{invalid}");
    }
}

#[test]
fn lexical_key_and_lookup_provenance_are_separate_from_valid_shadowing() {
    let mut regrouped = reduced();
    let body = root(&mut regrouped)["actions"].clone();
    root(&mut regrouped)["actions"] = json!([{"kind":"sequence","actions":[]},{"kind":"sequence","actions":body},{"kind":"sequence","actions":[]}]);
    rename(root(&mut regrouped), "key", "renamed_key");
    rename(root(&mut regrouped), "method", "renamed_value");
    included(&parse(regrouped), "verify");
    let mut shadowed = reduced();
    rename(root(&mut shadowed), "key", "id");
    let c = parse(shadowed);
    assert!(old(&c, "verify").unwrap().is_none());
    current(&c, "verify");
    let mut separate = reduced();
    root(&mut separate)["parameters"]
        .as_array_mut()
        .unwrap()
        .push(json!({"name":"otherId","ty":{"kind":"opaque_string"}}));
    key_let(&mut separate)["action"]["actions"][0]["condition"]["key"] =
        json!({"kind":"parameter","name":"otherId"});
    let c = parse(separate);
    assert!(old(&c, "verify").unwrap().is_none());
    current(&c, "verify");
    for invalid in ["name", "index", "scope"] {
        let mut v = reduced();
        let projection = &mut call(&mut v)["arguments"][1]["value"];
        match invalid {
            "name" => projection["field"] = json!("missing"),
            "index" => projection["index"] = json!(1),
            "scope" => projection["value"]["name"] = json!("missing_local"),
            _ => unreachable!(),
        }
        let c = parse(v);
        assert!(old(&c, "verify").unwrap().is_none());
        assert!(crate::render_with_capabilities(&c).is_err(), "{invalid}");
        assert!(checked(&c, "verify").is_err(), "{invalid}");
    }
}
