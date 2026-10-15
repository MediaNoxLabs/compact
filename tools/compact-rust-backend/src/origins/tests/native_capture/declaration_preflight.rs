//! Actual issuer/binder/native owner controls; genuine source probes run separately.
use super::*;
use crate::RenderError;

#[test]
fn fatal_declaration_name_uses_original_name_slot_without_changing_located_cause() {
    let mut ir = pure(field());
    ir["circuits"][0]["name"] = json!("crate");
    ir["circuits"][0]["source"] = json!({"file":"declaration.compact","line":3,"column":1});
    let seed = Sandbox::new(&ir, None, None, 0);
    let metadata = raw(&ir, &seed.source);
    let expected = address(
        &metadata,
        &[
            json!({"field":"circuits"}),
            json!({"index":0}),
            json!({"field":"name"}),
        ],
    );
    let sandbox = Sandbox::new(&ir, Some(&serde_json::to_vec(&metadata).unwrap()), None, 0);
    let failed = failure(&sandbox);
    let selected = diagnostic(&failed);
    assert_eq!(selected.address_id, expected);
    assert_eq!(selected.selected_slot, "name");
    assert!(matches!(
        failed.cause,
        NativeFailureCause::Render(RenderError::Located { .. })
    ));
}

#[test]
fn named_type_identifier_selects_original_type_before_definition_clone() {
    let mut ir = pure(field());
    ir["circuits"][0]["parameters"] = json!([{"name":"value","ty":{"kind":"struct","name":"crate","fields":[{"name":"value","ty":{"kind":"field"}}]}}]);
    let seed = Sandbox::new(&ir, None, None, 0);
    let metadata = raw(&ir, &seed.source);
    let expected = address(
        &metadata,
        &[
            json!({"field":"circuits"}),
            json!({"index":0}),
            json!({"field":"parameters"}),
            json!({"index":0}),
            json!({"field":"ty"}),
            json!({"field":"name"}),
        ],
    );
    let sandbox = Sandbox::new(&ir, Some(&serde_json::to_vec(&metadata).unwrap()), None, 0);
    let failed = failure(&sandbox);
    assert_eq!(diagnostic(&failed).address_id, expected);
    assert_eq!(diagnostic(&failed).selected_slot, "name");
}

#[test]
fn local_depth_failure_captures_prospective_left_child_not_current_parent() {
    let parameter = || missing("x");
    let mut expression = parameter();
    for _ in 0..50 {
        expression = json!({"kind":"subtract","left":parameter(),"right":expression});
    }
    let mut ir = pure(expression);
    ir["circuits"][0]["parameters"] = json!([{"name":"x","ty":{"kind":"field"}}]);
    let seed = Sandbox::new(&ir, None, None, 0);
    let metadata = raw(&ir, &seed.source);
    let mut parent_path = body();
    for _ in 0..46 {
        parent_path.push(json!({"field":"right"}));
    }
    let parent = address(&metadata, &parent_path);
    parent_path.push(json!({"field":"left"}));
    let prospective = address(&metadata, &parent_path);
    let sandbox = Sandbox::new(&ir, Some(&serde_json::to_vec(&metadata).unwrap()), None, 0);
    let failed = failure(&sandbox);
    assert!(matches!(
        failed.cause,
        NativeFailureCause::Render(RenderError::ResourceLimit {
            resource: "syntax_depth",
            observed: 49,
            limit: 48
        })
    ));
    assert_eq!(diagnostic(&failed).address_id, prospective);
    assert_ne!(prospective, parent);
}

#[test]
fn cumulative_string_failure_uses_explicit_assert_message_slot() {
    let mut ir = pure(
        json!({"kind":"sequence","steps":[{"kind":"assert","condition":boolean(),"message":"m".repeat(262145)}],"value":missing("x")}),
    );
    ir["circuits"][0]["parameters"] = json!([{"name":"x","ty":{"kind":"field"}}]);
    let seed = Sandbox::new(&ir, None, None, 0);
    let metadata = raw(&ir, &seed.source);
    let mut path = body();
    path.extend([
        json!({"field":"steps"}),
        json!({"index":0}),
        json!({"field":"message"}),
    ]);
    let expected = address(&metadata, &path);
    let sandbox = Sandbox::new(&ir, Some(&serde_json::to_vec(&metadata).unwrap()), None, 0);
    let failed = failure(&sandbox);
    assert!(matches!(
        failed.cause,
        NativeFailureCause::Render(RenderError::ResourceLimit {
            resource: "string_bytes",
            observed: 262146,
            limit: 262144
        })
    ));
    assert_eq!(diagnostic(&failed).address_id, expected);
    assert_eq!(diagnostic(&failed).selected_slot, "message");
}

#[test]
fn unmapped_declared_name_and_unwitnessed_final_path_remain_unavailable() {
    let mut ir = pure(field());
    ir["circuits"][0]["name"] = json!("crate");
    let seed = Sandbox::new(&ir, None, None, 0);
    let mut metadata = raw(&ir, &seed.source);
    let name = address(
        &metadata,
        &[
            json!({"field":"circuits"}),
            json!({"index":0}),
            json!({"field":"name"}),
        ],
    );
    metadata["addresses"][name][3] = Value::Null;
    let sandbox = Sandbox::new(&ir, Some(&serde_json::to_vec(&metadata).unwrap()), None, 0);
    assert!(failure(&sandbox).diagnostic.is_none());

    let mut expression = field();
    for _ in 0..43 {
        expression = json!({"kind":"transient_hash","value":expression});
    }
    let ir = pure(expression);
    let metadata = raw(&ir, &seed.source);
    let sandbox = Sandbox::new(&ir, Some(&serde_json::to_vec(&metadata).unwrap()), None, 0);
    let failed = failure(&sandbox);
    assert!(matches!(
        failed.cause,
        NativeFailureCause::Render(RenderError::ResourceLimit {
            resource: "pure_render_path_units",
            ..
        })
    ));
    assert!(
        failed.diagnostic.is_none(),
        "no last-visited-node blame for aggregate frame check"
    );
}
