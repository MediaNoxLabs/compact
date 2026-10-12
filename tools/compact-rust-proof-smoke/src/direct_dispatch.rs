// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Direct-selector routing, with positional fallback for every unmatched value.
//!
//! Matching never consumes arguments. A selected command owns argument checks,
//! proof execution, and any additional worker thread required by that scenario.

mod collections;
mod contracts;
mod expressions;
mod identity;
mod material;
mod merkle;
mod recorded;
mod shielded;

use std::error::Error;
use std::ffi::{OsStr, OsString};

use super::asset_dispatch::AssetProof;

pub(super) fn try_run(
    first: Option<&OsStr>,
    arguments: &mut impl Iterator<Item = OsString>,
) -> Option<Result<(), Box<dyn Error>>> {
    let selector = first?.to_str()?;
    let asset = AssetProof::from_selector(first);
    match (selector, asset) {
        ("--prepare-proof-material", _) => Some(material::prepare_proof_material(arguments)),
        ("--mixed-width-recording", _) => Some(expressions::mixed_width_recording(arguments)),
        ("--mixed-width-record-pinned", _) => {
            Some(expressions::mixed_width_record_pinned(arguments))
        }
        ("--wide-unsigned-field-cast", _) => Some(expressions::wide_unsigned_field_cast(arguments)),
        ("--closed-pure-unsigned-call", _) => {
            Some(expressions::closed_pure_unsigned_call(arguments))
        }
        ("--test-center-counter", _) => Some(contracts::test_center_counter(arguments)),
        ("--test-center-welcome", _) => Some(contracts::test_center_welcome(arguments)),
        ("--test-center-welcome-check-in", _) => {
            Some(contracts::test_center_welcome_check_in(arguments))
        }
        ("--test-center-welcome-organizers", _) => {
            Some(contracts::test_center_welcome_organizers(arguments))
        }
        ("--opaque-string-set", _) => Some(collections::opaque_string_set(arguments)),
        ("--opaque-string-set-member", _) => Some(collections::opaque_string_set_member(arguments)),
        ("--opaque-string-map", _) => Some(collections::opaque_string_map(arguments)),
        ("--opaque-string-map-ensure", _) => Some(collections::opaque_string_map_ensure(arguments)),
        ("--map-effects", _) => Some(collections::map_effects(arguments)),
        ("--map-value-products", _) => Some(collections::map_value_products(arguments)),
        ("--unit-composition", _) => Some(collections::unit_composition(arguments)),
        ("--aligned-hashes", _) => Some(collections::aligned_hashes(arguments)),
        ("--exact-hash-input", _) => Some(collections::exact_hash_input(arguments)),
        ("--jubjub-negation", _) => Some(collections::jubjub_negation(arguments)),
        ("--unit-value-operations", _) => Some(collections::unit_value_operations(arguments)),
        ("--aligned-witness-results", _) => Some(collections::aligned_witness_results(arguments)),
        ("--conditional-set", _) => Some(collections::conditional_set(arguments)),
        ("--adt-set-enum", _) => Some(collections::adt_set_enum(arguments)),
        ("--guarded-recording", _) => Some(expressions::guarded_recording(arguments)),
        (_, Some(proof)) => Some(run_asset(proof, arguments)),
        ("--schnorr-attestation", _) => Some(expressions::schnorr_attestation(arguments)),
        ("--zswap-transfer", _) => Some(shielded::zswap_transfer(arguments)),
        ("--shielded-receive", _) => Some(shielded::shielded_receive(arguments)),
        ("--shielded-merge", _) => Some(shielded::shielded_merge(arguments)),
        ("--shielded-send", _) => Some(shielded::shielded_send(arguments)),
        ("--observational-receive" | "--observational-send", _) => {
            Some(shielded::observational(first, arguments))
        }
        ("--observational-shielded", _) => Some(shielded::observational_shielded(arguments)),
        ("--wallet-funded-receive", _) => Some(shielded::wallet_funded_receive(arguments)),
        ("--kernel-shielded-effects", _) => Some(shielded::kernel_shielded_effects(arguments)),
        ("--qualified-coin-set", _) => Some(shielded::qualified_coin_set(arguments)),
        ("--qualified-coin-set-chunked", _) => {
            Some(shielded::qualified_coin_set_chunked(arguments))
        }
        ("--qualified-coin-cell", _) => Some(shielded::qualified_coin_cell(arguments)),
        ("--adt-set-qualified-coin-info", _) => {
            Some(shielded::adt_set_qualified_coin_info(arguments))
        }
        ("--adt-set-vector", _) => Some(collections::adt_set_vector(arguments)),
        ("--adt-set-vector-strict", _) => Some(collections::adt_set_vector_strict(arguments)),
        ("--adt-list-vector-field-4", _) => Some(collections::adt_list_vector_field_4(arguments)),
        ("--adt-list-bytes", _) => Some(collections::adt_list_bytes(arguments)),
        ("--adt-list-enum", _) => Some(collections::adt_list_enum(arguments)),
        ("--adt-list-field", _) => Some(collections::adt_list_field(arguments)),
        ("--adt-list-field-strict", _) => Some(collections::adt_list_field_strict(arguments)),
        ("--map-remaining-strict", _) => Some(collections::map_remaining_strict(arguments)),
        ("--closed-pure-field", _) => Some(expressions::closed_pure_field(arguments)),
        ("--struct-constructor-cell", _) => Some(expressions::struct_constructor_cell(arguments)),
        ("--pure-assert-call", _) => Some(expressions::pure_assert_call(arguments)),
        ("--field-pair-hash", _) => Some(expressions::field_pair_hash(arguments)),
        ("--impure-field-helper", _) => Some(expressions::impure_field_helper(arguments)),
        ("--unused-field-reads", _) => Some(expressions::unused_field_reads(arguments)),
        ("--let-return", _) => Some(expressions::let_return(arguments)),
        ("--field-to-bytes32", _) => Some(expressions::field_to_bytes32(arguments)),
        ("--literal-bytes-field", _) => Some(expressions::literal_bytes_field(arguments)),
        ("--literal-bytes-field-strict", _) => {
            Some(expressions::literal_bytes_field_strict(arguments))
        }
        ("--coracle-start", _) => Some(contracts::coracle_start(arguments)),
        ("--coracle-concede", _) => Some(contracts::coracle_concede(arguments)),
        ("--coracle-withdraw", _) => Some(contracts::coracle_withdraw(arguments)),
        ("--coracle-guess", _) => Some(contracts::coracle_guess(arguments)),
        ("--transient-receive-send", _) => Some(shielded::transient_receive_send(arguments)),
        ("--canonical-output-order", _) => Some(shielded::canonical_output_order(arguments)),
        ("--did-relation-lifecycle", _) => Some(identity::did_relation_lifecycle(arguments)),
        ("--did-relation-schnorr", _) => Some(identity::did_relation_schnorr(arguments)),
        ("--did-point-lifecycle", _) => Some(identity::did_point_lifecycle(arguments)),
        ("--did-alias-lifecycle", _) => Some(identity::did_alias_lifecycle(arguments)),
        ("--did-service-lifecycle", _) => Some(identity::did_service_lifecycle(arguments)),
        ("--did-schnorr-method-lifecycle", _) => {
            Some(identity::did_schnorr_method_lifecycle(arguments))
        }
        ("--did-jwk-method-lifecycle", _) => Some(identity::did_jwk_method_lifecycle(arguments)),
        ("--did-primitive-reducer", _) => Some(identity::did_primitive_reducer(arguments)),
        ("--did-digest-reducer", _) => Some(identity::did_digest_reducer(arguments)),
        ("--did-digest-verification", _) => Some(identity::did_digest_verification(arguments)),
        ("--did-deactivate", _) => Some(identity::did_deactivate(arguments)),
        ("--terminal-lexical-return", _) => Some(expressions::terminal_lexical_return(arguments)),
        ("--micro-dao-buy-in", _) => Some(contracts::micro_dao_buy_in(arguments)),
        ("--micro-dao-cash-out", _) => Some(contracts::micro_dao_cash_out(arguments)),
        ("--micro-dao-vote-commit", _) => Some(contracts::micro_dao_vote_commit(arguments)),
        ("--micro-dao-set-topic", _) => Some(contracts::micro_dao_set_topic(arguments)),
        ("--micro-dao-advance", _) => Some(contracts::micro_dao_advance(arguments)),
        ("--micro-dao-reveal", _) => Some(contracts::micro_dao_reveal(arguments)),
        ("--micro-dao-token", _) => Some(contracts::micro_dao_token(arguments)),
        ("--composite-zswap", _) => Some(shielded::composite_zswap(arguments)),
        ("--field-observation", _) => Some(expressions::field_observation(arguments)),
        ("--opaque-public", _) => Some(expressions::opaque_public(arguments)),
        ("--opaque-cell-input", _) => Some(expressions::opaque_cell_input(arguments)),
        ("--owned-witness-inputs", _) => Some(expressions::owned_witness_inputs(arguments)),
        ("--public-products", _) => Some(expressions::public_products(arguments)),
        ("--wide-add", _) => Some(expressions::wide_add(arguments)),
        ("--wide-casts", _) => Some(expressions::wide_casts(arguments)),
        ("--stateful-struct", _) => Some(recorded::stateful_struct(arguments)),
        ("--stateful-assert", _) => Some(recorded::stateful_assert(arguments)),
        ("--effect-plan-unsigned-arithmetic", _) => {
            Some(recorded::effect_plan_unsigned_arithmetic(arguments))
        }
        ("--effect-plan-merkle", _) => Some(recorded::effect_plan_merkle(arguments)),
        ("--effect-plan-transient-crypto", _) => {
            Some(recorded::effect_plan_transient_crypto(arguments))
        }
        ("--effect-plan-counter", _) => Some(recorded::effect_plan_counter(arguments)),
        ("--effect-plan-counter-only", _) => Some(recorded::effect_plan_counter_only(arguments)),
        ("--effect-plan-jubjub", _) => Some(recorded::effect_plan_jubjub(arguments)),
        ("--effect-plan-rich-values", _) => Some(recorded::effect_plan_rich_values(arguments)),
        ("--effect-plan-pure-calls", _) => Some(recorded::effect_plan_pure_calls(arguments)),
        ("--effect-plan-stateful-calls", _) => {
            Some(recorded::effect_plan_stateful_calls(arguments))
        }
        ("--guarded-point-read", _) => Some(recorded::guarded_point_read(arguments)),
        ("--guarded-point-nested", _) => Some(recorded::guarded_point_nested(arguments)),
        ("--guarded-point-vm", _) => Some(recorded::guarded_point_vm(arguments)),
        ("--effectful-return", _) => Some(recorded::effectful_return(arguments)),
        ("--effect-plan-wide-source", _) => Some(recorded::effect_plan_wide_source(arguments)),
        #[cfg(feature = "experimental-recording-plan")]
        ("--effect-plan-prototype", _) => Some(recorded::effect_plan_prototype(arguments)),
        #[cfg(not(feature = "experimental-recording-plan"))]
        ("--effect-plan-prototype", _) => Some(Err(
            "--effect-plan-prototype requires the experimental-recording-plan Cargo feature".into(),
        )),
        ("--root-let-action-return", _) => Some(recorded::root_let_action_return(arguments)),
        ("--counter-less-than", _) => Some(recorded::counter_less_than(arguments)),
        ("--pair-hash-cell", _) => Some(recorded::pair_hash_cell(arguments)),
        ("--stateful-pair-hash", _) => Some(recorded::stateful_pair_hash(arguments)),
        ("--boolean-pair-assert", _) => Some(expressions::boolean_pair_assert(arguments)),
        ("--pure-field-arguments", _) => Some(expressions::pure_field_arguments(arguments)),
        ("--stateful-pure-return", _) => Some(recorded::stateful_pure_return(arguments)),
        ("--persistent-commit", _) => Some(expressions::persistent_commit(arguments)),
        ("--persistent-commit-strict", _) => Some(expressions::persistent_commit_strict(arguments)),
        ("--assert-witness", _) => Some(recorded::assert_witness(arguments)),
        ("--witness-vector-let", _) => Some(recorded::witness_vector_let(arguments)),
        ("--witness-vector-bare", _) => Some(recorded::witness_vector_bare(arguments)),
        ("--witness-vector-discard", _) => Some(recorded::witness_vector_discard(arguments)),
        ("--composite-cell", _) => Some(recorded::composite_cell(arguments)),
        ("--boolean-observation", _) => Some(recorded::boolean_observation(arguments)),
        ("--conditional-counter", _) => Some(recorded::conditional_counter(arguments)),
        ("--unsigned-sequential", _) => Some(recorded::unsigned_sequential(arguments)),
        ("--conditional-field", _) => Some(recorded::conditional_field(arguments)),
        ("--conditional-assert-eq", _) => Some(recorded::conditional_assert_eq(arguments)),
        ("--conditional-compare-eq", _) => Some(recorded::conditional_compare_eq(arguments)),
        ("--nested-uint64-cell-counter", _) => {
            Some(recorded::nested_uint64_cell_counter(arguments))
        }
        ("--nested-uint4", _) => Some(recorded::nested_uint4(arguments)),
        ("--conditional-field-vector", _) => Some(recorded::conditional_field_vector(arguments)),
        ("--annotated-uint8", _) => Some(recorded::annotated_uint8(arguments)),
        ("--native-curve-arg", _) => Some(recorded::native_curve_arg(arguments)),
        ("--conditional-struct-member", _) => Some(recorded::conditional_struct_member(arguments)),
        ("--merkle-verify", _) => Some(merkle::merkle_verify(arguments)),
        ("--merkle-reset", _) => Some(merkle::merkle_reset(arguments)),
        ("--vector-map", _) => Some(collections::vector_map(arguments)),
        ("--merkle-root", _) => Some(merkle::merkle_root(arguments)),
        ("--zerocash-spend", _) => Some(contracts::zerocash_spend(arguments)),
        ("--bboard", _) => Some(contracts::bboard(arguments)),
        ("--election-reveal", _) => Some(contracts::election_reveal(arguments)),
        ("--election-commit", _) => Some(contracts::election_commit(arguments)),
        ("--election-add-voter", _) => Some(contracts::election_add_voter(arguments)),
        ("--election-advance", _) => Some(contracts::election_advance(arguments)),
        ("--zerocash-mint", _) => Some(contracts::zerocash_mint(arguments)),
        ("--election-topic", _) => Some(contracts::election_topic(arguments)),
        ("--inline-type-scope", _) => Some(expressions::inline_type_scope(arguments)),
        ("--merkle-hash", _) => Some(merkle::merkle_hash(arguments)),
        ("--historic-merkle-hash", _) => Some(merkle::historic_merkle_hash(arguments)),
        ("--historic-merkle-indexed-hash", _) => {
            Some(merkle::historic_merkle_indexed_hash(arguments))
        }
        ("--historic-merkle-reset-history", _) => {
            Some(merkle::historic_merkle_reset_history(arguments))
        }
        ("--historic-merkle-reset-tree", _) => Some(merkle::historic_merkle_reset_tree(arguments)),
        ("--historic-merkle-known", _) => Some(merkle::historic_merkle_known(arguments)),
        ("--merkle-indexed", _) => Some(merkle::merkle_indexed(arguments)),
        ("--historic-merkle-indexed", _) => Some(merkle::historic_merkle_indexed(arguments)),
        _ => None,
    }
}

fn run_asset(
    proof: AssetProof,
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = proof.parse_root(arguments)?;
    proof.run(&root)
}

#[cfg(test)]
mod tests {
    use super::try_run;
    use std::ffi::{OsStr, OsString};

    #[test]
    fn positional_fallback_never_reads_the_remaining_arguments() {
        for first in [
            None,
            Some(""),
            Some("counter output"),
            Some("--unknown"),
            Some("--asset"),
        ] {
            let mut arguments = std::iter::from_fn(|| panic!("fallback consumed an argument"));
            assert!(try_run(first.map(OsStr::new), &mut arguments).is_none());
        }
    }

    #[test]
    fn malformed_root_counts_keep_usage_and_single_extra_argument_lookahead() {
        // All paths deliberately fail before proof work or worker creation.
        for (selector, arity, usage) in [
            (
                "--mixed-width-recording",
                1,
                "usage: compact-rust-proof-smoke --mixed-width-recording <proof-output>",
            ),
            (
                "--opaque-public",
                1,
                "usage: compact-rust-proof-smoke --opaque-public <proof-output>",
            ),
            (
                "--owned-witness-inputs",
                1,
                "usage: compact-rust-proof-smoke --owned-witness-inputs <proof-output>",
            ),
            (
                "--public-products",
                1,
                "usage: compact-rust-proof-smoke --public-products <proof-output>",
            ),
            (
                "--wide-add",
                1,
                "usage: compact-rust-proof-smoke --wide-add <proof-output>",
            ),
            (
                "--wide-casts",
                1,
                "usage: compact-rust-proof-smoke --wide-casts <proof-output>",
            ),
            (
                "--wide-unsigned-field-cast",
                1,
                "usage: compact-rust-proof-smoke --wide-unsigned-field-cast <proof-output>",
            ),
            (
                "--historic-merkle-indexed",
                2,
                "usage: compact-rust-proof-smoke --historic-merkle-indexed <insert-output> <default-output>",
            ),
            (
                "--pure-field-arguments",
                2,
                "usage: compact-rust-proof-smoke --pure-field-arguments <internal-proof> <ternary-proof>",
            ),
            (
                "--vector-map",
                2,
                "usage: --vector-map <identity-output> <arithmetic-constructor-output>",
            ),
            (
                "--asset-freshness",
                1,
                "usage: compact-rust-proof-smoke --asset-freshness <proof-output>",
            ),
        ] {
            for available in 0..arity {
                let mut supplied =
                    (0..available).map(|index| OsString::from(format!("root-{index}")));
                let mut calls = 0;
                let mut counted = std::iter::from_fn(|| {
                    calls += 1;
                    supplied.next()
                });
                let error = try_run(Some(OsStr::new(selector)), &mut counted)
                    .expect("recognized selector")
                    .unwrap_err();
                assert_eq!(error.to_string(), usage, "{selector}");
                assert_eq!(calls, available + 1, "{selector}");
            }
            let mut arguments = (0..arity + 2).map(|index| OsString::from(format!("root-{index}")));
            let error = try_run(Some(OsStr::new(selector)), &mut arguments)
                .expect("recognized selector")
                .unwrap_err();
            assert_eq!(error.to_string(), usage, "{selector}");
            assert_eq!(
                arguments.next(),
                Some(OsString::from(format!("root-{}", arity + 1)))
            );
        }
    }

    #[test]
    fn observational_aliases_keep_their_distinct_argument_errors() {
        for selector in ["--observational-receive", "--observational-send"] {
            let missing = try_run(Some(OsStr::new(selector)), &mut std::iter::empty())
                .unwrap()
                .unwrap_err();
            assert_eq!(missing.to_string(), "expected observational proof root");
            let mut arguments = ["root", "extra", "unconsumed"]
                .map(OsString::from)
                .into_iter();
            let excess = try_run(Some(OsStr::new(selector)), &mut arguments)
                .unwrap()
                .unwrap_err();
            assert_eq!(
                excess.to_string(),
                "unexpected observational proof argument"
            );
            assert_eq!(arguments.next(), Some(OsString::from("unconsumed")));
        }
    }

    #[test]
    fn three_root_command_reports_the_first_missing_root_without_drain() {
        for (available, expected) in [
            "expected seven proof output",
            "expected eight-a proof output",
            "expected eight-b proof output",
        ]
        .into_iter()
        .enumerate()
        {
            let mut arguments = (0..available).map(|index| OsString::from(format!("root-{index}")));
            let error = try_run(Some(OsStr::new("--unused-field-reads")), &mut arguments)
                .unwrap()
                .unwrap_err();
            assert_eq!(error.to_string(), expected);
        }
        let mut arguments = ["seven", "eight-a", "eight-b", "extra", "unconsumed"]
            .map(OsString::from)
            .into_iter();
        let error = try_run(Some(OsStr::new("--unused-field-reads")), &mut arguments)
            .unwrap()
            .unwrap_err();
        assert_eq!(error.to_string(), "unexpected unused-field-reads argument");
        assert_eq!(arguments.next(), Some(OsString::from("unconsumed")));
    }

    #[test]
    fn material_option_errors_stop_before_material_preparation() {
        let usage = "usage: compact-rust-proof-smoke --prepare-proof-material [--verify-only]";
        for supplied in [
            vec!["--invalid", "unconsumed"],
            vec!["--verify-only", "extra", "unconsumed"],
        ] {
            let mut arguments = supplied.into_iter().map(OsString::from);
            let error = try_run(Some(OsStr::new("--prepare-proof-material")), &mut arguments)
                .unwrap()
                .unwrap_err();
            assert_eq!(error.to_string(), usage);
            assert_eq!(arguments.next(), Some(OsString::from("unconsumed")));
        }
    }

    #[cfg(unix)]
    #[test]
    fn non_utf8_selector_falls_back_without_consuming_or_rewriting_paths() {
        use std::os::unix::ffi::OsStringExt;

        let first = OsString::from_vec(b"--asset-freshness\xff".to_vec());
        let next = OsString::from_vec(vec![b'p', 0xff]);
        let mut arguments = std::iter::once(next.clone());
        assert!(try_run(Some(first.as_os_str()), &mut arguments).is_none());
        assert_eq!(first, OsString::from_vec(b"--asset-freshness\xff".to_vec()));
        assert_eq!(arguments.next(), Some(next));
    }
}
