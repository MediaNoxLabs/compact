// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Exact mixed public product encoding through a real funded call proof.
use super::*;
use compact_rust_effect_plan_public_products_source_fixture::{
    RETAINED_IR, ledger_contract as c, ledger_slots as slots, types,
};
use midnight_compact_runtime::{
    self as runtime,
    fab::{Alignment, AlignmentAtom},
    recording::RecordingFrame,
};
use std::{cell::Cell, rc::Rc};

const TEXT: &str = "é文\0tail";
const NESTED: &str = "nested🦀";
const PAIR_BYTES: &[u8] = &[1, 0, 255];
const FIRST: &[u8] = &[2, 0, 3];
const SECOND: &[u8] = &[4, 0, 5];
const REPLACEMENT: &[u8] = &[9, 0, 8];

#[derive(Clone, Default)]
struct Witness(Rc<Cell<usize>>);
impl Witness {
    fn observe(&self, private: u64, counter: u128) -> (u64, runtime::OpaqueBytes) {
        assert_eq!((private, counter), (5, 1));
        self.0.set(self.0.get() + 1);
        (6, REPLACEMENT.to_vec().into())
    }
}
impl c::TryWitnesses<u64> for Witness {
    fn replacement(
        &self,
        context: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, runtime::OpaqueBytes), runtime::CompactError> {
        Ok(self.observe(*context.private_state, context.ledger.calls()?.value()))
    }
}

fn wide_bytes() -> [u8; 17] {
    let mut bytes = [0; 17];
    bytes[0] = 37;
    bytes[16] = 1; // 2^128 + 37: genuinely above the small unsigned carrier.
    bytes
}

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    // Complete source binding precedes key reads/state/funding; not key authentication.
    counter_scenario::validate_ir(
        &std::fs::read(root.join("contract/compact-rust-ir.json"))?,
        RETAINED_IR,
    )?;
    let name = "run";
    let mut rng = StdRng::seed_from_u64(0x0558_0000);
    let initial = c::initial_state(ConstructorContext::new(5_u64))?;
    let deploy = make_deploy(root, name, initial.ledger_state.get_ref().clone(), &mut rng)?;
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
        root.join("keys/run.verifier"),
    )?))?;
    let input = types::Product {
        pair: (TEXT.into(), PAIR_BYTES.to_vec().into()),
        items: runtime::FixedVector::new([FIRST.to_vec().into(), SECOND.to_vec().into()]),
        nested: types::Nested {
            tag: Field::from(9_u64),
            text: NESTED.into(),
        },
        wide: runtime::WideUint::from_le_bytes(&wide_bytes())?,
        empty: types::Empty {},
    };
    let (native_witness, recorded_witness, observed_witness, manual_witness) = (
        Witness::default(),
        Witness::default(),
        Witness::default(),
        Witness::default(),
    );
    let native = c::run(
        observed.circuit_context(5),
        &native_witness,
        input.clone(),
        true,
    )?;
    let recorded = c::recorded::run(
        observed.circuit_context(5),
        &recorded_witness,
        input.clone(),
        true,
    )?;
    let generated = c::Contract::from(observed_witness.clone());
    let call = generated
        .recording()
        .run_call(&observed, 5, input.clone(), true)?;
    let frame =
        slots::calls.record_increment(RecordingFrame::new(observed.circuit_context(5)), 1)?;
    let (frame, replacement) = frame.try_witness_metered(|context, meter| {
        let count = slots::calls.witness_read(meter)?;
        Ok(manual_witness.observe(context.private_state, count as u128))
    })?;
    let chosen = types::Product {
        items: runtime::FixedVector::new([PAIR_BYTES.to_vec().into(), replacement]),
        ..input.clone()
    };
    let manual = frame.finish(types::Result {
        chosen,
        original: input.clone(),
    });
    for witness in [
        &native_witness,
        &recorded_witness,
        &observed_witness,
        &manual_witness,
    ] {
        if witness.0.get() != 1 {
            return Err("public product witness count differs".into());
        }
    }
    for candidate in [&recorded, call.recorded(), &manual] {
        if native.result != candidate.execution.result
            || native.context.query.state != candidate.execution.context.query.state
            || native.context.query.effects != candidate.execution.context.query.effects
            || native.gas_cost != candidate.execution.gas_cost
            || native.context.private_state != 6
            || candidate.execution.context.private_state != 6
            || native.private_transcript_outputs != candidate.execution.private_transcript_outputs
            || candidate.execution.private_transcript_outputs
                != vec![AlignedValue::from(runtime::OpaqueBytes::from(
                    REPLACEMENT.to_vec(),
                ))]
            || candidate.public.verify_ops() != manual.public.verify_ops()
            || candidate.public.verify_ops().is_empty()
        {
            return Err("public product native/recorded/manual execution differs".into());
        }
        let replay = candidate.public.initial().query(
            candidate.public.verify_ops(),
            None,
            &candidate.execution.context.cost_model,
        )?;
        if replay.context.state != native.context.query.state
            || replay.context.effects != native.context.query.effects
        {
            return Err("public product transcript replay differs".into());
        }
    }
    if slots::calls.inspect(native.context.query.state.get_ref())? != 1 {
        return Err("public product Counter differs from source".into());
    }
    let expected_state = native.context.query.state.get_ref().clone();
    let prepared = call.prepare(verifier.clone(), Fr::from(0_u64))?;
    check_encoding(&prepared)?;
    let direct = check_generated_trace(root, name, recorded, (input, true))?;
    if format!("{direct:?}") != format!("{prepared:?}") {
        return Err("public product manual/observed preparation differs".into());
    }
    kernel_shielded_effects::prove_and_verify_call(root, name, &prepared, &verifier)?;
    opaque_public::apply_funded(root, name, prepared, deploy, &expected_state, rng, 558)?;
    println!(
        "public products: literal tuple/vector/wide/empty atoms and alignments, replay, proof binding refusal and funded default-strict application passed"
    );
    Ok(())
}

fn check_encoding(prepared: &ContractCallPrototype<DefaultDB>) -> Result<(), Box<dyn Error>> {
    let wide = wide_bytes();
    let product_atoms = |first: &[u8], second: &[u8]| {
        [
            TEXT.as_bytes(),
            PAIR_BYTES,
            first,
            second,
            &[9],
            NESTED.as_bytes(),
            &wide,
        ]
        .into_iter()
        .map(<[u8]>::to_vec)
        .collect::<Vec<_>>()
    };
    let mut input = product_atoms(FIRST, SECOND);
    input.push(vec![1]);
    let mut output = product_atoms(PAIR_BYTES, REPLACEMENT);
    output.extend(product_atoms(FIRST, SECOND));
    let atoms = |value: &AlignedValue| {
        value
            .value
            .0
            .iter()
            .map(|v| v.0.clone())
            .collect::<Vec<_>>()
    };
    // Empty{} contributes no atom; the wide value contributes exactly one17-byte atom.
    if atoms(&prepared.input) != input || atoms(&prepared.output) != output {
        return Err("public product atoms differ from literal source-order bytes".into());
    }
    let product = [
        AlignmentAtom::Compress,
        AlignmentAtom::Compress,
        AlignmentAtom::Compress,
        AlignmentAtom::Compress,
        AlignmentAtom::Field,
        AlignmentAtom::Compress,
        AlignmentAtom::Bytes { length: 17 },
    ]
    .map(Alignment::singleton);
    let product = Alignment::concat(product.iter());
    let boolean = Alignment::singleton(AlignmentAtom::Bytes { length: 1 });
    if prepared.input.alignment != Alignment::concat([&product, &boolean])
        || prepared.output.alignment != Alignment::concat([&product, &product])
    {
        return Err("public product declared alignment differs".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn full_source_change_refuses() {
        let mut changed: serde_json::Value = serde_json::from_str(RETAINED_IR).unwrap();
        changed["independent_guard_change"] = serde_json::json!(true);
        assert!(
            counter_scenario::validate_ir(&serde_json::to_vec(&changed).unwrap(), RETAINED_IR)
                .is_err()
        );
    }
    #[test]
    fn missing_source_refuses_before_keys_or_state() {
        let root = std::env::temp_dir().join(format!(
            "compact-product-missing-ir-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        assert!(!root.exists());
        let error = run(&root).unwrap_err();
        assert_eq!(
            error.downcast_ref::<io::Error>().unwrap().kind(),
            io::ErrorKind::NotFound
        );
        assert!(!root.exists());
    }
}
