// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Official checks against source-produced ZKIR and independent ledger programs.
//! These are source/VM controls, not generated-recording capability or proofs.
use super::*;
use r::context::{CircuitContext, ConstructorResult};
use r::ledger::{CellValue, ChargedState, constructor_cell};
use r::recording::RecordingFrame;
use r::slots::CellSlot;

fn frame<T: CellValue>(value: T) -> RecordingFrame<u64> {
    let state = StateValue::Array(vec![constructor_cell::<_, DefaultDB>(value)].into());
    let context: CircuitContext<u64> =
        ConstructorResult::new(ConstructorContext::new(7), ChargedState::new(state))
            .into_circuit_context(Default::default());
    RecordingFrame::new(context)
}

fn preimage(
    root: &Path,
    name: &'static str,
    recorded: RecordedCircuitResult<u64, r::JubjubPoint>,
    input: AlignedValue,
) -> Result<ProofPreimage, Box<dyn Error>> {
    // A verifier carrier satisfies preparation's typed API. No proof is made or
    // checked against this key in this VM-only route.
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join("carrier.verifier"),
    )?))?;
    let call = prepare_call(
        recorded,
        CallSpec::new(name, verifier, input, Fr::from(0_u64)),
    )?;
    let mut fields = Vec::new();
    call.input.value_only_field_repr(&mut fields);
    call.output.value_only_field_repr(&mut fields);
    let commitment = transient_commit(&fields[..], call.communication_commitment_rand);
    match <ProofPreimage as ContractCallExt<DefaultDB>>::construct_proof(&call, commitment) {
        ProofPreimageVersioned::V2(preimage) => Ok(preimage.as_ref().clone()),
        _ => Err("unsupported source VM preimage version".into()),
    }
}

fn ir(root: &Path, case: &str, name: &str) -> Result<IrSource, Box<dyn Error>> {
    Ok(serde_json::from_slice(&std::fs::read(
        root.join(format!("{case}/zkir/{name}.zkir")),
    )?)?)
}

pub(crate) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    let point = r::ec_mul_generator(Field::from(7_u64))?;
    let other = r::ec_mul_generator(Field::from(9_u64))?;
    let result_slot = CellSlot::<r::JubjubPoint>::new(&[0]);
    for selected in [false, true] {
        let f = frame(r::JubjubPoint::identity());
        let (f, supplied) = f.witness(|_| (8, point));
        let expected = if selected {
            r::ec_add(supplied, r::ec_mul_generator(Field::from(1_u64))?)
        } else {
            r::ec_mul(supplied, Field::from(2_u64))?
        };
        let f = result_slot.record_write(f, expected)?;
        let (f, result) = result_slot.record_read(f)?;
        let pre = preimage(
            root,
            "from_witness",
            f.finish(result),
            AlignedValue::from(selected),
        )?;
        pre.check(&ir(root, "witness", "from_witness")?)?;
        println!(
            "source witness selected={selected}: official VM checked active witness and inactive point operand"
        );

        // Envelope/Inner/vector contain the same six aligned Field atoms as this
        // independent tuple; the source reads the whole Cell before projection.
        let stored = (point, other, Field::from(11_u64), Field::from(13_u64));
        let f = frame(stored);
        let (f, read) =
            CellSlot::<(r::JubjubPoint, r::JubjubPoint, Field, Field)>::new(&[0]).record_read(f)?;
        let expected = if selected {
            r::ec_add(read.0, r::ec_mul_generator(Field::from(1_u64))?)
        } else {
            r::ec_mul(read.1, Field::from(2_u64))?
        };
        let pre = preimage(
            root,
            "from_aggregate",
            f.finish(expected),
            AlignedValue::from(selected),
        )?;
        pre.check(&ir(root, "aggregate", "from_aggregate")?)?;
        println!(
            "source nested aggregate selected={selected}: official VM checked six-atom read/projection"
        );
    }
    let invalid_ir = ir(root, "invalid", "invalid")?;
    for selected in [false, true] {
        let expected = if selected {
            point
        } else {
            r::JubjubPoint::identity()
        };
        let f = result_slot.record_write(frame(r::JubjubPoint::identity()), expected)?;
        let (f, read) = result_slot.record_read(f)?;
        let input =
            AlignedValue::concat(&[AlignedValue::from(selected), AlignedValue::from(point)]);
        preimage(root, "invalid", f.finish(read), input)?.check(&invalid_ir)?;
        println!(
            "source construct-point selected={selected}: official VM checked valid coordinates"
        );
    }
    // Inactive malformed coordinates must remain unused. Active malformed
    // coordinates are diagnosed separately because the pinned official host
    // constructor can panic before producing a structured check error.
    let f = result_slot.record_write(
        frame(r::JubjubPoint::identity()),
        r::JubjubPoint::identity(),
    )?;
    let (f, read) = result_slot.record_read(f)?;
    let input = AlignedValue::concat(&[
        AlignedValue::from(false),
        AlignedValue::from(Field::from(0_u64)),
        AlignedValue::from(Field::from(0_u64)),
    ]);
    let inactive = preimage(root, "invalid", f.finish(read), input)?;
    inactive.check(&invalid_ir)?;
    println!(
        "source construct-point inactive (0,0): official VM accepted selected identity branch"
    );
    // Keep source, guard and public program synchronized for active rejection.
    // Reuse the identity result program: rejection must happen at the active
    // malformed curve operand, independently of output commitment mismatch.
    let f = result_slot.record_write(
        frame(r::JubjubPoint::identity()),
        r::JubjubPoint::identity(),
    )?;
    let (f, read) = result_slot.record_read(f)?;
    let active_input = AlignedValue::concat(&[
        AlignedValue::from(true),
        AlignedValue::from(Field::from(0_u64)),
        AlignedValue::from(Field::from(0_u64)),
    ]);
    let active = preimage(root, "invalid", f.finish(read), active_input)?;
    match std::panic::catch_unwind(|| active.check(&invalid_ir)) {
        Ok(Ok(_)) => return Err("source accepted active malformed coordinates".into()),
        Ok(Err(error)) => {
            if !error
                .to_string()
                .contains("Elliptic curve point not on curve")
            {
                return Err(format!(
                    "active malformed point failed for an unrelated reason: {error}"
                )
                .into());
            }
            println!("source active malformed coordinates refused: {error}");
        }
        Err(payload) => {
            let message = payload
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| payload.downcast_ref::<&str>().copied())
                .unwrap_or("");
            if !message.contains("Point should be part of the subgroup") {
                return Err("active malformed point caused an unrelated host panic".into());
            }
            println!(
                "source active malformed (0,0): pinned host constructor panicked; diagnostic refusal only"
            );
        }
    }
    Ok(())
}
