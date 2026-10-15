// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Small synthetic official-VM controls, separate from source-matched proofs.
//! A pinned dependency panic is recorded as a host failure, not proof rejection.
use super::*;
use midnight_zkir::Instruction;

fn projection(default: (Fr, Fr)) -> IrSource {
    use Instruction::*;
    IrSource {
        num_inputs: 4,
        do_communications_commitment: true,
        instructions: vec![
            ConstrainToBoolean { var: 0 },
            ConstrainToBoolean { var: 1 },
            Mul { a: 0, b: 1 },                // 4: effective nested guard
            LoadImm { imm: default.0 },        // 5
            LoadImm { imm: default.1 },        // 6
            CondSelect { bit: 4, a: 2, b: 5 }, // 7
            CondSelect { bit: 4, a: 3, b: 6 }, // 8
            LoadImm {
                imm: Fr::from(1_u64),
            }, // 9
            EcMul {
                a_x: 7,
                a_y: 8,
                scalar: 9,
            }, // 10,11
            Output { var: 10 },
            Output { var: 11 },
        ]
        .into(),
    }
}

fn preimage(outer: bool, inner: bool, point: (Fr, Fr), output: (Fr, Fr)) -> ProofPreimage {
    let inputs = vec![
        Fr::from(u64::from(outer)),
        Fr::from(u64::from(inner)),
        point.0,
        point.1,
    ];
    let mut committed = inputs.clone();
    committed.extend([output.0, output.1]);
    ProofPreimage {
        inputs,
        private_transcript: vec![],
        public_transcript_inputs: vec![],
        public_transcript_outputs: vec![],
        binding_input: Fr::from(0_u64),
        communications_commitment: Some((
            transient_commit(&committed[..], Fr::from(0_u64)),
            Fr::from(0_u64),
        )),
        key_location: KeyLocation("synthetic-projection-check-only".into()),
    }
}

fn coordinates(point: r::JubjubPoint) -> (Fr, Fr) {
    (
        point.x().unwrap_or(Fr::from(0_u64)),
        point.y().unwrap_or(Fr::from(0_u64)),
    )
}

#[test]
fn official_vm_uses_identity_for_every_inactive_nested_guard() {
    let ir = projection((Fr::from(0_u64), Fr::from(1_u64)));
    let identity = coordinates(r::JubjubPoint::identity());
    for (outer, inner) in [(false, false), (false, true), (true, false)] {
        let malformed = preimage(outer, inner, (Fr::from(0_u64), Fr::from(0_u64)), identity);
        assert!(malformed.check(&ir).is_ok(), "guard {outer}/{inner}");
    }
    let valid = coordinates(r::ec_mul_generator(Field::from(7_u64)).unwrap());
    assert!(preimage(true, true, valid, valid).check(&ir).is_ok());
}

#[test]
fn official_vm_rejects_a_changed_valid_default_or_guard() {
    let identity = coordinates(r::JubjubPoint::identity());
    let valid = coordinates(r::ec_mul_generator(Field::from(7_u64)).unwrap());
    let inactive = preimage(false, true, valid, identity);
    assert!(
        inactive.check(&projection(valid)).is_err(),
        "wrong valid default changed committed output"
    );
    let ir = projection((Fr::from(0_u64), Fr::from(1_u64)));
    let mut flipped = inactive.clone();
    flipped.inputs[0] = Fr::from(1_u64);
    assert!(
        flipped.check(&ir).is_err(),
        "flipped guard must change committed statement"
    );
    flipped.inputs[0] = Fr::from(2_u64);
    assert!(flipped.check(&ir).is_err(), "non-Boolean guard must fail");
}

#[test]
fn active_malformed_point_remains_an_official_host_refusal() {
    let ir = projection((Fr::from(0_u64), Fr::from(1_u64)));
    let input = preimage(
        true,
        true,
        (Fr::from(0_u64), Fr::from(0_u64)),
        coordinates(r::JubjubPoint::identity()),
    );
    match std::panic::catch_unwind(|| input.check(&ir)) {
        Ok(Ok(_)) => panic!("active malformed coordinates were silently sanitized"),
        Ok(Err(_)) => (),
        Err(_) => eprintln!(
            "pinned official curve constructor panics on active (0,0); host refusal only, not a verified proof rejection"
        ),
    }
}
