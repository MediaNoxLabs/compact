//! Original header and carrier sites, with exact existing native causes.
use super::*;
use crate::RenderError;

fn issued(ir: &Value, path: &[Value]) -> (NativeCompileFailure, usize) {
    let seed = Sandbox::new(ir, None, None, 0);
    let metadata = raw(ir, &seed.source);
    let expected = address(&metadata, path);
    let sandbox = Sandbox::new(ir, Some(&serde_json::to_vec(&metadata).unwrap()), None, 0);
    (*failure(&sandbox), expected)
}
fn circuit_parameter() -> Vec<Value> {
    vec![
        json!({"field":"circuits"}),
        json!({"index":0}),
        json!({"field":"parameters"}),
        json!({"index":0}),
    ]
}

#[test]
fn fatal_parameter_name_wins_over_bad_carrier_after_allocator_probe() {
    let mut ir = pure(field());
    ir["circuits"][0]["parameters"] =
        json!([{"name":"crate","ty":{"kind":"unsigned","max":"bad"}}]);
    let mut path = circuit_parameter();
    path.push(json!({"field":"name"}));
    let (failed, expected) = issued(&ir, &path);
    assert!(
        matches!(failed.cause,NativeFailureCause::Render(RenderError::InvalidIdentifier(ref name)) if name=="crate")
    );
    assert_eq!(diagnostic(&failed).address_id, expected);
    assert_eq!(diagnostic(&failed).selected_slot, "name");
}

#[test]
fn witness_and_ledger_names_capture_their_original_declarations() {
    let mut witness_ir = fixture::empty_ir();
    witness_ir["witnesses"] = json!([{"name":"crate","parameters":[],"result":{"kind":"field"}}]);
    let (failed, expected) = issued(
        &witness_ir,
        &[
            json!({"field":"witnesses"}),
            json!({"index":0}),
            json!({"field":"name"}),
        ],
    );
    assert_eq!(diagnostic(&failed).address_id, expected);
    assert!(
        matches!(failed.cause,NativeFailureCause::Render(RenderError::InvalidIdentifier(ref name)) if name=="crate")
    );
    let mut ledger_ir = fixture::empty_ir();
    ledger_ir["ledger_fields"] =
        json!([{"id":"crate","index":0,"path":[0],"declaration":{"kind":"counter"}}]);
    let (failed, expected) = issued(
        &ledger_ir,
        &[
            json!({"field":"ledger_fields"}),
            json!({"index":0}),
            json!({"field":"id"}),
        ],
    );
    assert_eq!(diagnostic(&failed).address_id, expected);
    assert_eq!(diagnostic(&failed).selected_slot, "id");
}

#[test]
fn alias_identifier_and_existing_reserved_namespace_conflict_are_name_owned() {
    for (name, conflict) in [("crate", false), ("runtime", true)] {
        let mut ir = fixture::empty_ir();
        ir["type_aliases"] = json!([{"name":name,"ty":{"kind":"field"}}]);
        let (failed, expected) = issued(
            &ir,
            &[
                json!({"field":"type_aliases"}),
                json!({"index":0}),
                json!({"field":"name"}),
            ],
        );
        assert_eq!(diagnostic(&failed).address_id, expected);
        if conflict {
            assert!(
                matches!(failed.cause,NativeFailureCause::Render(RenderError::ConflictingTypeAlias(ref value)) if value==name)
            );
        } else {
            assert!(
                matches!(failed.cause,NativeFailureCause::Render(RenderError::InvalidIdentifier(ref value)) if value==name)
            );
        }
    }
}

#[test]
fn nested_carrier_maximum_selects_original_first_leaf_once() {
    let mut ir = pure(field());
    ir["circuits"][0]["parameters"] = json!([{"name":"value","ty":{"kind":"tuple","elements":[{"kind":"unsigned","max":"bad-first"},{"kind":"unsigned","max":"bad-second"}]}}]);
    let mut path = circuit_parameter();
    path.extend([
        json!({"field":"ty"}),
        json!({"field":"elements"}),
        json!({"index":0}),
        json!({"field":"max"}),
    ]);
    let (failed, expected) = issued(&ir, &path);
    assert_eq!(diagnostic(&failed).address_id, expected);
    assert_eq!(diagnostic(&failed).selected_slot, "max");
}

#[test]
fn unsupported_map_carrier_refuses_before_bad_descendant_and_keeps_type_payload() {
    let mut ir = pure(field());
    ir["circuits"][0]["parameters"] = json!([{"name":"value","ty":{"kind":"ledger_map","key":{"kind":"unsigned","max":"bad"},"value":{"kind":"field"}}}]);
    let mut path = circuit_parameter();
    path.push(json!({"field":"ty"}));
    let (failed, expected) = issued(&ir, &path);
    assert_eq!(diagnostic(&failed).address_id, expected);
    assert_eq!(diagnostic(&failed).selected_slot, "node");
    assert!(matches!(
        failed.cause,
        NativeFailureCause::Render(RenderError::UnsupportedLedgerValueType(
            crate::ir::Type::LedgerMap { .. }
        ))
    ));
}

#[test]
fn witness_result_and_alias_body_carriers_keep_original_type_identity() {
    let mut witness_ir = fixture::empty_ir();
    witness_ir["witnesses"] =
        json!([{"name":"probe","parameters":[],"result":{"kind":"unsigned","max":"bad"}}]);
    let (failed, expected) = issued(
        &witness_ir,
        &[
            json!({"field":"witnesses"}),
            json!({"index":0}),
            json!({"field":"result"}),
            json!({"field":"max"}),
        ],
    );
    assert_eq!(diagnostic(&failed).address_id, expected);
    let mut alias_ir = fixture::empty_ir();
    alias_ir["type_aliases"] = json!([{"name":"Score","ty":{"kind":"unsigned","max":"bad"}}]);
    let (failed, expected) = issued(
        &alias_ir,
        &[
            json!({"field":"type_aliases"}),
            json!({"index":0}),
            json!({"field":"ty"}),
            json!({"field":"max"}),
        ],
    );
    assert_eq!(diagnostic(&failed).address_id, expected);
}
