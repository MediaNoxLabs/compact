// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

fn public_refusal_matches_checked(c: &Contract, gap: &RecordingGap) {
    let output = crate::render_with_capabilities(c).unwrap();
    let capability = output
        .capabilities
        .circuits
        .iter()
        .find(|c| c.name == "start")
        .unwrap();
    assert!(!capability.recorded && !capability.observed_call);
    assert_eq!(capability.recording_unavailable.as_ref().unwrap(), gap);
    assert_eq!(
        capability.observed_call_unavailable.as_ref().unwrap(),
        &RecordingGap::recording_dependency(gap.clone())
    );
}

#[test]
fn genuine_unsupported_child_and_native_invalid_authority_keep_distinct_failures() {
    let mut value = source();
    prefix_binding(
        &mut value,
        "unsupported",
        json!({"kind":"field"}),
        serde_json::to_value(test_support::unsupported_fixed_input()).unwrap(),
    );
    let mut c = parse(value);
    test_support::register_unsupported_fold(&mut c);
    assert!(
        old(&c, "start").is_none(),
        "direct Fold is not a complete old positive"
    );
    let Err(effect_plan::PlanError::Unsupported(gap)) = checked(&c, "start") else {
        panic!("expected direct Fold refusal");
    };
    assert_eq!(gap.ir_node, "Expr::VectorFoldCall");
    assert_eq!(gap.path, "return_value.body.bindings[0].value");
    let output = crate::render_with_capabilities(&c).unwrap();
    let cap = output
        .capabilities
        .circuits
        .iter()
        .find(|c| c.name == "start")
        .unwrap();
    assert!(!cap.recorded && !cap.observed_call);
    assert_eq!(cap.recording_unavailable.as_ref().unwrap(), &gap);

    let mut wrong_slot = source();
    let mut changed = false;
    edit(&mut root(&mut wrong_slot)["return_value"], &mut |node| {
        if !changed && node.get("kind") == Some(&json!("cell_write_coin")) {
            node.insert("field".into(), json!("state"));
            node.insert("index".into(), json!(5));
            changed = true;
        }
    });
    assert!(changed);
    let wrong_slot = parse(wrong_slot);
    assert!(old(&wrong_slot, "start").is_none());
    assert!(crate::render_with_capabilities(&wrong_slot).is_err());
    let mut wrong_witness = source();
    wrong_witness["witnesses"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|w| w["name"] == "fresh_nonce")
        .unwrap()["result"] = json!({"kind":"boolean"});
    let wrong_witness = parse(wrong_witness);
    assert!(old(&wrong_witness, "start").is_none());
    assert!(crate::render_with_capabilities(&wrong_witness).is_err());
    let mut recursive = source();
    recursive["circuits"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|c| c["name"] == "coinCommitment")
        .unwrap()["body"] = json!({"kind":"call","name":"coinCommitment","arguments":[]});
    let recursive = parse(recursive);
    assert!(old(&recursive, "start").is_none());
    assert!(crate::render_with_capabilities(&recursive).is_err());
    let mut bad_name = parse(source());
    bad_name
        .stateful_circuits
        .iter_mut()
        .find(|c| c.name == "start")
        .unwrap()
        .name = "invalid-root".into();
    assert!(
        old(&bad_name, "invalid-root").is_some(),
        "old plan does not emit root identifier"
    );
    assert!(crate::render_with_capabilities(&bad_name).is_err());
}

#[test]
fn resource_trait_and_source_policy_exclusions_are_not_claimed_as_source_inclusion() {
    // A small IR node can describe a large fixed value. The old generator has
    // no fixed-layout recording guard; this is an explicit budget-policy case,
    // not permission to relax the checked limit or a runtime memory promise.
    let mut oversized = source();
    let ty = json!({"kind":"struct","name":"OversizedFundingLocal","fields":[{
        "name":"items","ty":{"kind":"vector","element":{"kind":"field"},
        "length":crate::resource_limits::recording_fixed_layout_limit()}}]});
    prefix_binding(
        &mut oversized,
        "oversized",
        ty.clone(),
        json!({"kind":"default","ty":ty}),
    );
    let oversized = parse(oversized);
    assert!(old(&oversized, "start").is_some());
    let Err(effect_plan::PlanError::Unsupported(gap)) = checked(&oversized, "start") else {
        panic!("oversized fixed layout must stay refused");
    };
    assert_eq!(
        gap.resource.as_ref().unwrap().kind,
        crate::RecordingResourceKind::FixedLayoutSlots
    );
    assert_eq!(
        gap.resource.as_ref().unwrap().limit,
        crate::resource_limits::recording_fixed_layout_limit()
    );
    // Complete native rendering remains accepted, while the recorded APIs now
    // preserve the checked resource policy instead of using the old fallback.
    public_refusal_matches_checked(&oversized, &gap);

    let mut trait_excluded = source();
    let ty = json!({"kind":"struct","name":"TraitExcludedFundingLocal","fields":[{
        "name":"tuple","ty":{"kind":"tuple","elements":vec![json!({"kind":"field"});9]}}]});
    prefix_binding(
        &mut trait_excluded,
        "trait_excluded",
        ty.clone(),
        json!({"kind":"default","ty":ty}),
    );
    let trait_excluded = parse(trait_excluded);
    assert!(old(&trait_excluded, "start").is_some());
    let Err(effect_plan::PlanError::Unsupported(gap)) = checked(&trait_excluded, "start") else {
        panic!("unsupported named representation must remain refused");
    };
    assert!(
        gap.resource.is_none(),
        "trait eligibility is not a resource budget"
    );
    // Named CellValue derives stop at tuple8. This control is native-renderable
    // typed IR, not a Rust-compilable source contract compatibility promise.
    public_refusal_matches_checked(&trait_excluded, &gap);

    // Opaque persistent hashing is rejected by the Compact frontend. Existing
    // native pure implementation/certificate behavior remains a separate policy;
    // do not turn its syntax into a purported source-valid missing primitive.
    let mut native_only = source();
    native_only["circuits"].as_array_mut().unwrap().push(json!({
        "name":"native_only_opaque_hash","internal":true,"parameters":[],
        "result":{"kind":"bytes","length":32},"body":{"kind":"persistent_hash",
        "value":{"kind":"default","ty":{"kind":"opaque_string"}}}}));
    prefix_binding(
        &mut native_only,
        "native_only",
        json!({"kind":"bytes","length":32}),
        json!({"kind":"call","name":"native_only_opaque_hash","arguments":[]}),
    );
    included(&parse(native_only), "start");
}
