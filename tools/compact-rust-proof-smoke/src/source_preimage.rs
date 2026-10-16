// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Official preimage checks shared by exact-source proof adapters.
use super::*;

/// Use the official checker before proving. A changed guard must fail; a VM panic
/// remains a failed run and is never treated as a successful negative control.
pub(crate) fn check(
    root: &Path,
    circuit: &'static str,
    call: &ContractCallPrototype<DefaultDB>,
    branch: Option<bool>,
) -> Result<(), Box<dyn Error>> {
    let ir: IrSource = tagged_deserialize(&mut BufReader::new(File::open(
        root.join(format!("zkir/{circuit}.bzkir")),
    )?))?;
    let mut fields = Vec::new();
    call.input.value_only_field_repr(&mut fields);
    call.output.value_only_field_repr(&mut fields);
    let commitment = transient_commit(&fields[..], call.communication_commitment_rand);
    let preimage =
        match <ProofPreimage as ContractCallExt<DefaultDB>>::construct_proof(call, commitment) {
            ProofPreimageVersioned::V2(preimage) => preimage,
            _ => return Err("unexpected source preimage version".into()),
        };
    preimage.check(&ir)?;
    if let Some(selected) = branch {
        if preimage.inputs.first() != Some(&Fr::from(u64::from(selected))) {
            return Err("source branch guard is not first proof input".into());
        }
        for changed_guard in [Fr::from(u64::from(!selected)), Fr::from(2_u64)] {
            let mut changed = preimage.as_ref().clone();
            changed.inputs[0] = changed_guard;
            if changed.check(&ir).is_ok() {
                return Err("official checker accepted a tampered source branch guard".into());
            }
        }
        println!(
            "{circuit} selected={selected}: original preimage checked; flipped and non-Boolean guards rejected"
        );
    }
    Ok(())
}
