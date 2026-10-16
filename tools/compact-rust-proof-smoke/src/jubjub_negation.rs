// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Source-bound proof of Jubjub negation emitted by the recording effect algebra.
use super::*;
use compact_rust_effect_plan_jubjub_negation_source_fixture::{
    RETAINED_IR, ledger_contract as c, ledger_slots as slots, runtime as r, types,
};
use std::{cell::RefCell, rc::Rc};
type Event = (&'static str, u64, u128);
#[derive(Clone, Default)]
struct Witness(Rc<RefCell<Vec<Event>>>);
impl Witness {
    fn take(&self, name: &'static str, private: u64, count: u128) -> u64 {
        self.0.borrow_mut().push((name, private, count));
        private + 1
    }
}
impl c::TryWitnesses<u64> for Witness {
    fn point(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, r::JubjubPoint), r::CompactError> {
        Ok((
            self.take("point", *ctx.private_state, ctx.ledger.calls()?.value()),
            known_point(),
        ))
    }
    fn permit(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, bool), r::CompactError> {
        Ok((
            self.take("permit", *ctx.private_state, ctx.ledger.calls()?.value()),
            true,
        ))
    }
}
fn known_point() -> r::JubjubPoint {
    // Original TS coordinates-nonidentity coordinates. Do not compute the expected
    // public point through the Rust negation operation being qualified.
    let x = Field::from_le_bytes(&[
        157, 52, 201, 185, 204, 155, 101, 1, 10, 105, 33, 117, 104, 143, 190, 125, 12, 131, 178,
        249, 240, 240, 108, 254, 120, 107, 228, 167, 96, 185, 219, 18,
    ])
    .unwrap();
    let y = Field::from_le_bytes(&[
        17, 169, 199, 80, 148, 108, 92, 119, 108, 249, 200, 118, 27, 54, 239, 222, 17, 105, 237,
        204, 41, 209, 198, 215, 185, 85, 130, 220, 150, 114, 176, 75,
    ])
    .unwrap();
    let atoms = r::fab::Value::from((x, y));
    r::JubjubPoint::try_from(&*atoms).unwrap()
}
fn fallback_point() -> r::JubjubPoint {
    // Exact public fallback from the selected TS row, checked as a curve point.
    let atoms = r::fab::Value::from((Field::from(0_u64), Field::from(1_u64)));
    r::JubjubPoint::try_from(&*atoms).unwrap()
}
fn known_output() -> types::Coordinates {
    // Independent frozen TS output; never call Rust ec_neg to obtain it.
    types::Coordinates {
        x: Field::from_le_bytes(&[
            100, 203, 54, 70, 50, 100, 154, 254, 244, 242, 220, 138, 154, 20, 255, 213, 248, 84,
            239, 15, 23, 231, 204, 52, 207, 17, 185, 129, 242, 237, 17, 97,
        ])
        .unwrap(),
        y: Field::from_le_bytes(&[
            17, 169, 199, 80, 148, 108, 92, 119, 108, 249, 200, 118, 27, 54, 239, 222, 17, 105,
            237, 204, 41, 209, 198, 215, 185, 85, 130, 220, 150, 114, 176, 75,
        ])
        .unwrap(),
    }
}
pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    // Whole-source identity is checked before make_deploy can load proof keys.
    counter_scenario::validate_ir(
        &std::fs::read(root.join("contract/compact-rust-ir.json"))?,
        RETAINED_IR,
    )?;
    let name = "coordinates";
    let mut rng = StdRng::seed_from_u64(0x0585_0000);
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
    let a = Witness::default();
    let b = Witness::default();
    let w = Witness::default();
    let fallback = fallback_point();
    let expected = known_output();
    let native = c::coordinates(observed.circuit_context(5), &a, fallback, true)?;
    let recorded = c::recorded::coordinates(observed.circuit_context(5), &b, fallback, true)?;
    let contract = c::Contract::from(w.clone());
    let call = contract
        .recording()
        .coordinates_call(&observed, 5, fallback, true)?;
    let private = vec![AlignedValue::from(known_point()), AlignedValue::from(true)];
    for witness in [&a, &b, &w] {
        if *witness.0.borrow() != vec![("point", 5, 1), ("permit", 6, 1)] {
            return Err("Jubjub negation callback order/read/private state mismatch".into());
        }
    }
    if native.result != expected
        || native.context.private_state != 7
        || native.private_transcript_outputs != private
    {
        return Err("Jubjub negation native output/private FAB differs from frozen TS".into());
    }
    for candidate in [&recorded, call.recorded()] {
        if candidate.execution.result != expected
            || candidate.execution.context.private_state != 7
            || candidate.execution.private_transcript_outputs != private
            || candidate.execution.context.query.state != native.context.query.state
            || candidate.execution.context.query.effects != native.context.query.effects
            || candidate.execution.gas_cost != native.gas_cost
            || candidate.public.verify_ops() != recorded.public.verify_ops()
            || candidate.public.verify_ops().is_empty()
        {
            return Err("Jubjub negation native/recorded/observed binding differs".into());
        }
    }
    let expected_state = native.context.query.state.get_ref().clone();
    if slots::calls.inspect(&expected_state)? != 1
        || slots::calls.inspect(observed.circuit_context(5).query.state.get_ref())? != 0
    {
        return Err("Jubjub negation Counter or immutable observation differs".into());
    }
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join("keys/coordinates.verifier"),
    )?))?;
    let prepared = call.prepare(verifier.clone(), Fr::from(0_u64))?;
    let input = (fallback, true);
    if prepared.input != AlignedValue::from(input)
        || prepared.output != AlignedValue::from(expected)
    {
        return Err("Jubjub negation prepared original input/product output differs".into());
    }
    // Ordinary shared preparation replays and checks the generated trace;
    // exhaustive manual/replay cases live in the source fixture.
    let direct = check_generated_trace(root, name, recorded, input)?;
    if format!("{direct:?}") != format!("{prepared:?}") {
        return Err("Jubjub negation direct/observed preparation differs".into());
    }
    kernel_shielded_effects::prove_and_verify_call(root, name, &prepared, &verifier)?;
    opaque_public::apply_funded(root, name, prepared, deploy, &expected_state, rng, 585)?;
    println!(
        "coordinates(nonidentity, selected=true): algebra.composite-values recorded EcNeg, frozen TS x/y output and public identity fallback, ordered point/permit witnesses, exact private FAB, Counter, default-v2 verified proof+changed-binding refusal and default-strict funded ledger8 application; constructor state supplied, deployment not proved"
    );
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn aligned_json(value: &AlignedValue) -> serde_json::Value {
        serde_json::json!({"value":value.value.0.iter().map(|a|&a.0).collect::<Vec<_>>(),"alignment":value.alignment})
    }
    #[test]
    fn mismatched_whole_source_refuses_before_keys() {
        counter_scenario::validate_ir(RETAINED_IR.as_bytes(), RETAINED_IR).unwrap();
        let mut value: serde_json::Value = serde_json::from_str(RETAINED_IR).unwrap();
        value["independent_jubjub_negation_guard"] = true.into();
        assert!(
            counter_scenario::validate_ir(&serde_json::to_vec(&value).unwrap(), RETAINED_IR)
                .is_err()
        );
    }
    #[test]
    fn coordinates_input_output_and_private_outputs_match_frozen_ts() {
        let oracle: serde_json::Value = serde_json::from_str(include_str!(
            "../../../tests-rust-backend/effect-plan-jubjub-negation-source/oracle/typescript.json"
        ))
        .unwrap();
        let rows = oracle["rows"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["id"] == "coordinates-nonidentity")
            .collect::<Vec<_>>();
        assert_eq!(rows.len(), 1);
        let row = rows[0];
        assert_eq!(row["circuit"], "coordinates");
        assert_eq!(row["args"], serde_json::json!([row["fallback"], true]));
        assert_eq!(row["selected"], true);
        assert_eq!(row["permit"], true);
        assert_eq!(row["fail"], serde_json::Value::Null);
        assert_eq!(row["status"], "success");
        assert_eq!(row["serializedPreimage"]["status"], "success");
        assert_eq!(
            row["events"],
            serde_json::json!([["point", 5, "1"], ["permit", 6, "1"]])
        );
        assert_eq!(row["privateState"], 7);
        assert_eq!(row["fallback"], serde_json::json!({"x":"0","y":"1"}));
        assert_eq!(row["result"], row["expected"]);
        assert_eq!(
            aligned_json(&AlignedValue::from((fallback_point(), true))),
            row["proofData"]["input"]
        );
        assert_eq!(
            aligned_json(&AlignedValue::from(known_output())),
            row["proofData"]["output"]
        );
        let private = [AlignedValue::from(known_point()), AlignedValue::from(true)];
        assert_eq!(
            serde_json::Value::Array(private.iter().map(aligned_json).collect()),
            row["proofData"]["privateTranscriptOutputs"]
        );
    }
}
