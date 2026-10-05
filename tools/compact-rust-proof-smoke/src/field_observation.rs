// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Read-only composites: real nonempty proofs and exact empty-call refusal.
use super::*;
use compact_rust_field_observation_oracle_fixture::ledger_contract as c;
use midnight_compact_runtime as runtime;
pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    for (index, (name, selected)) in [
        ("direct", false),
        ("snapshot", false),
        ("via_field_helper", false),
        ("via_snapshot_helper", false),
        ("pair", false),
        ("selected", false),
        ("selected", true),
        ("optional", true),
    ]
    .into_iter()
    .enumerate()
    {
        let mut rng = StdRng::seed_from_u64(0x0201_0000 + index as u64);
        let initial = c::initial_state(
            ConstructorContext::new(Vec::<u8>::new()),
            runtime::Field::from(7u64),
            runtime::Field::from(19u64),
        )?;
        let expected_state = initial.ledger_state.get_ref().clone();
        let deploy = make_deploy(root, name, expected_state.clone(), &mut rng)?;
        let observed = ObservedContractState::new(
            deploy.address(),
            deploy.initial_state.clone(),
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: 0,
            },
        );
        let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
            root.join(format!("keys/{name}.verifier")),
        )?))?;
        macro_rules! calls { ($circuit:ident, $call:ident, $input:expr $(,$arg:expr)*) => {{
            let recorded = c::recorded::$circuit(observed.circuit_context(Vec::<u8>::new()) $(,$arg)*)?;
            let manual = check_generated_trace(root, name, recorded, $input)?;
            let typed = c::Contract::default().recording.$call(&observed, Vec::<u8>::new() $(,$arg)*)?;
            (manual, typed.prepare(verifier.clone(), Fr::from(0u64))?)
        }}; }
        let (manual, prepared) = match name {
            "direct" => calls!(direct, direct_call, ()),
            "snapshot" => calls!(snapshot, snapshot_call, ()),
            "via_field_helper" => calls!(via_field_helper, via_field_helper_call, ()),
            "via_snapshot_helper" => calls!(via_snapshot_helper, via_snapshot_helper_call, ()),
            "pair" => calls!(pair, pair_call, ()),
            "selected" => calls!(selected, selected_call, selected, selected),
            "optional" => {
                let empty = c::Contract::default().recording.optional_call(
                    &observed,
                    Vec::<u8>::new(),
                    false,
                )?;
                if !matches!(
                    empty.prepare(verifier.clone(), Fr::from(0u64)),
                    Err(ObservedCallError::Prepare(
                        runtime::transaction::PrepareCallError::EmptyTranscript
                    ))
                ) {
                    return Err("optional(false) must retain exact empty-transcript refusal".into());
                }
                println!("optional(false): exact EmptyTranscript, no fabricated operation");
                calls!(optional, optional_call, true, true)
            }
            _ => unreachable!(),
        };
        if format!("{manual:?}") != format!("{prepared:?}") {
            return Err("manual/observed preparation differs".into());
        }
        check_transaction(root, name, deploy, prepared, &mut rng, |state| {
            if state.data.get_ref() != &expected_state {
                return Err("read-only observation changed ledger state".into());
            }
            Ok(())
        })?;
        println!(
            "{name}({selected}): proved, verified, ledger-applied under shared unbalanced smoke policy"
        );
    }
    Ok(())
}
