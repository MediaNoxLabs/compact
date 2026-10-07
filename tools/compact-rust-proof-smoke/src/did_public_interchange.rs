// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Optional public-only interchange fixture for a separately qualified ledger reader.
use super::*;
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

pub(super) fn capture_if_requested(
    transaction: &PublicTransaction,
    before: &LedgerState<DefaultDB>,
    context: &TransactionContext<DefaultDB>,
    operation: &str,
) -> Result<(), Box<dyn Error>> {
    if operation != "rotateControllerKey" {
        return Ok(());
    }
    let Some(directory) = env::var_os("COMPACT_DID_PUBLIC_INTERCHANGE") else {
        return Ok(());
    };
    let directory = PathBuf::from(directory);
    if !directory.is_absolute() {
        return Err("public DID interchange requires an absolute new directory".into());
    }
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
    // Never replace an earlier receipt or write TestState, proof preimages, wallet keys,
    // witness/private-state payloads or RNG state. These concrete types are public.
    fs::create_dir(&directory)?;
    write_public(&directory.join("transaction.bin"), transaction)?;
    write_public(&directory.join("ledger-before.bin"), before)?;
    write_public(&directory.join("ledger-after.bin"), &after)?;
    write_public(&directory.join("block-context.bin"), &context.block_context)?;
    fs::write(
        directory.join("context.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "format": "compact-did-public-interchange/v1",
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
        }))?,
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
}
