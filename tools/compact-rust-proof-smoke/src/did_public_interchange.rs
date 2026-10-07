// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Optional public-only interchange fixture for a separately qualified ledger reader.
use super::*;
use midnight_ledger::structure::ContractAction;
use midnight_serialize::{Serializable, Tagged};

type PublicTransaction = Transaction<Signature, ProofMarker, PureGeneratorPedersen, DefaultDB>;

fn write_public<T: Serializable + Tagged>(path: &Path, value: &T) -> Result<(), Box<dyn Error>> {
    let mut bytes = Vec::new();
    tagged_serialize(value, &mut bytes)?;
    fs::write(path, bytes)?;
    Ok(())
}

fn validate_context(
    before: &LedgerState<DefaultDB>,
    context: &TransactionContext<DefaultDB>,
) -> Result<(), &'static str> {
    if context.whitelist.is_some() {
        return Err("public DID interchange does not support a whitelist");
    }
    if context.ref_state != *before {
        return Err("public DID interchange reference state differs from pre-ledger");
    }
    Ok(())
}

fn validate_case(scenario: &str, case: &str, operation: &str) -> Result<(), &'static str> {
    if matches!(
        (scenario, case, operation),
        ("points", "rotate", "rotateControllerKey")
            | ("points", "recover", "recoverControllerKey")
            | ("points", "deactivate", "deactivate")
            | (
                "aliases",
                "insert-unicode" | "remove-unicode",
                "setAlsoKnownAs"
            )
            | (
                "services",
                "insert-unicode" | "update-empty-fields",
                "setService"
            )
            | ("services", "remove-unicode", "removeService")
            | (
                "schnorr-methods",
                "insert-unicode" | "update-point",
                "setSchnorrJubjubVerificationMethod"
            )
            | (
                "schnorr-methods",
                "remove-unicode",
                "removeSchnorrJubjubVerificationMethod"
            )
            | (
                "jwk-methods",
                "insert-unicode" | "update-jwk",
                "setVerificationMethod"
            )
            | ("jwk-methods", "remove-unicode", "removeVerificationMethod")
            | ("digest", "insert", "setSchnorrJubjubVerificationMethod")
            | ("digest", "read-valid", "verifySchnorrJubjubDigestSignature")
    ) {
        Ok(())
    } else {
        Err("public DID interchange case is outside the reviewed matrix")
    }
}

fn require_unchanged_contract(
    before: &ContractState<DefaultDB>,
    after: &ContractState<DefaultDB>,
) -> Result<(), Box<dyn Error>> {
    let mut before_bytes = Vec::new();
    let mut after_bytes = Vec::new();
    tagged_serialize(before, &mut before_bytes)?;
    tagged_serialize(after, &mut after_bytes)?;
    if before_bytes != after_bytes {
        return Err("read-only DID digest call changed serialized contract state".into());
    }
    Ok(())
}

fn validate_read_only_digest(
    transaction: &PublicTransaction,
    before: &LedgerState<DefaultDB>,
    after: &LedgerState<DefaultDB>,
) -> Result<(), Box<dyn Error>> {
    let Transaction::Standard(standard) = transaction else {
        return Err("read-only DID digest requires a standard transaction".into());
    };
    let mut calls = 0;
    for entry in standard.intents.iter() {
        for action in entry.1.actions.iter_deref() {
            if let ContractAction::Call(call) = action {
                calls += 1;
                if call.entry_point.0 != b"verifySchnorrJubjubDigestSignature" {
                    return Err("read-only DID digest operation differs".into());
                }
                require_unchanged_contract(
                    before
                        .contract
                        .get(&call.address)
                        .ok_or("missing before contract")?,
                    after
                        .contract
                        .get(&call.address)
                        .ok_or("missing after contract")?,
                )?;
            }
        }
    }
    if calls != 1 {
        return Err("read-only DID digest requires exactly one call".into());
    }
    Ok(())
}

pub(super) fn capture_if_requested(
    transaction: &PublicTransaction,
    before: &LedgerState<DefaultDB>,
    context: &TransactionContext<DefaultDB>,
    scenario: &str,
    case: &str,
    operation: &str,
) -> Result<(), Box<dyn Error>> {
    let Some(directory) = env::var_os("COMPACT_DID_PUBLIC_INTERCHANGE") else {
        return Ok(());
    };
    let directory = PathBuf::from(directory);
    if !directory.is_absolute() {
        return Err("public DID interchange requires an absolute new directory".into());
    }
    validate_case(scenario, case, operation)?;
    validate_context(before, context)?;
    // Capture the exact apply result, before TestState::apply performs its post-block step.
    let verified = transaction.well_formed(
        before,
        WellFormedStrictness::default(),
        context.block_context.tblock,
    )?;
    let (after, result) = before.apply(&verified, context);
    if !matches!(result, TransactionResult::Success(_)) {
        return Err("public DID interchange transaction did not apply strictly".into());
    }
    let read_only_digest = scenario == "digest" && case == "read-valid";
    if read_only_digest {
        validate_read_only_digest(transaction, before, &after)?;
    }
    // Never replace an earlier receipt or write TestState, proof preimages, wallet keys,
    // witness/private-state payloads or RNG state. These concrete types are public.
    fs::create_dir_all(&directory)?;
    let directory = directory.join(format!("{scenario}--{case}"));
    fs::create_dir(&directory)?;
    write_public(
        &directory.join("producer-initial-parameters.bin"),
        &INITIAL_PARAMETERS,
    )?;
    write_public(&directory.join("transaction.bin"), transaction)?;
    write_public(&directory.join("ledger-before.bin"), before)?;
    write_public(&directory.join("ledger-after.bin"), &after)?;
    write_public(&directory.join("block-context.bin"), &context.block_context)?;
    let mut metadata = serde_json::json!({
        "format": "compact-did-public-interchange/v2",
        "scenario": scenario,
        "case": case,
        "producer_ledger": "8.0.3",
        "operation": operation,
        "network_id": before.network_id,
        "whitelist": null,
        "block_context": context.block_context,
        "strictness": "default",
        "constructor_data_deployed": true,
        "constructor_execution_proved": false,
        "post_block_update_included": false,
        "private_material_exported": false,
    });
    if read_only_digest {
        metadata["read_only_contract_unchanged"] = serde_json::json!(true);
    }
    fs::write(
        directory.join("context.json"),
        serde_json::to_vec_pretty(&metadata)?,
    )?;
    println!("public DID interchange written to {}", directory.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (LedgerState<DefaultDB>, TransactionContext<DefaultDB>) {
        let ledger = LedgerState::new("local-test");
        let context = TransactionContext {
            ref_state: ledger.clone(),
            block_context: BlockContext::default(),
            whitelist: None,
        };
        (ledger, context)
    }

    #[test]
    fn exact_reference_and_unrestricted_context_are_supported() {
        let (ledger, context) = fixture();
        assert_eq!(validate_context(&ledger, &context), Ok(()));
    }

    #[test]
    fn whitelist_cannot_be_mislabeled_as_unrestricted() {
        let (ledger, mut context) = fixture();
        context.whitelist = Some(Default::default());
        assert_eq!(
            validate_context(&ledger, &context),
            Err("public DID interchange does not support a whitelist")
        );
    }

    #[test]
    fn different_reference_cannot_be_replaced_by_preledger() {
        let (ledger, mut context) = fixture();
        context.ref_state.network_id = "other-public-network".into();
        assert_eq!(
            validate_context(&ledger, &context),
            Err("public DID interchange reference state differs from pre-ledger")
        );
    }
    #[test]
    fn reviewed_case_identity_rejects_cross_scenario_and_unknown_operations() {
        assert_eq!(
            validate_case("services", "update-empty-fields", "setService"),
            Ok(())
        );
        for (scenario, case, operation) in [
            ("services", "update-empty-fields", "removeService"),
            ("aliases", "update-empty-fields", "setAlsoKnownAs"),
            ("../escape", "rotate", "rotateControllerKey"),
            ("points", "rotate", "unknown"),
        ] {
            assert_eq!(
                validate_case(scenario, case, operation),
                Err("public DID interchange case is outside the reviewed matrix")
            );
        }
    }
    #[test]
    fn digest_rows_are_exact_and_do_not_cross_operations() {
        assert_eq!(
            validate_case("digest", "insert", "setSchnorrJubjubVerificationMethod"),
            Ok(())
        );
        assert_eq!(
            validate_case("digest", "read-valid", "verifySchnorrJubjubDigestSignature"),
            Ok(())
        );
        for (case, operation) in [
            ("insert", "verifySchnorrJubjubDigestSignature"),
            ("read-valid", "setSchnorrJubjubVerificationMethod"),
            ("read-invalid", "verifySchnorrJubjubDigestSignature"),
        ] {
            assert!(validate_case("digest", case, operation).is_err());
        }
    }
    #[test]
    fn read_only_guard_compares_complete_serialized_contract_state() {
        let state = |flag| {
            ContractState::new(
                midnight_compact_runtime::ledger::contract_state(vec![
                    midnight_compact_runtime::ledger::constructor_cell(flag),
                ])
                .get_ref()
                .clone(),
                Default::default(),
                ContractMaintenanceAuthority::default(),
            )
        };
        let before = state(true);
        require_unchanged_contract(&before, &before.clone()).unwrap();
        assert!(require_unchanged_contract(&before, &state(false)).is_err());
        let mut changed_operation = before.clone();
        changed_operation.operations = changed_operation.operations.insert(
            EntryPointBuf(b"extra".to_vec()),
            ContractOperation::new(None),
        );
        assert!(require_unchanged_contract(&before, &changed_operation).is_err());
    }
    #[test]
    fn prior_fourteen_reviewed_rows_remain_allowed() {
        for (scenario, case, operation) in [
            ("points", "rotate", "rotateControllerKey"),
            ("points", "recover", "recoverControllerKey"),
            ("points", "deactivate", "deactivate"),
            ("aliases", "insert-unicode", "setAlsoKnownAs"),
            ("aliases", "remove-unicode", "setAlsoKnownAs"),
            ("services", "insert-unicode", "setService"),
            ("services", "update-empty-fields", "setService"),
            ("services", "remove-unicode", "removeService"),
            (
                "schnorr-methods",
                "insert-unicode",
                "setSchnorrJubjubVerificationMethod",
            ),
            (
                "schnorr-methods",
                "update-point",
                "setSchnorrJubjubVerificationMethod",
            ),
            (
                "schnorr-methods",
                "remove-unicode",
                "removeSchnorrJubjubVerificationMethod",
            ),
            ("jwk-methods", "insert-unicode", "setVerificationMethod"),
            ("jwk-methods", "update-jwk", "setVerificationMethod"),
            ("jwk-methods", "remove-unicode", "removeVerificationMethod"),
        ] {
            assert_eq!(validate_case(scenario, case, operation), Ok(()));
        }
    }
}
