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

mod support;
use compact_rust_counter_parameter_fixture::ledger_contract as counter;
use midnight_base_crypto::{cost_model::CostDuration, time::Timestamp};
use midnight_coin_structure::coin::{PublicAddress, TokenType};
use midnight_compact_testkit::runtime::{
    BoundedUint, CompactError,
    context::{CircuitContext, CircuitFrame, ConstructorContext, RunningCost},
    ledger::{CoinCommitment, ContractAddress, HashOutput},
    recording::RecordingFrame,
};
use midnight_compact_testkit::{ContractLab, LabError, WitnessScript};

type Context = CircuitContext<Vec<String>>;
type Mutation = (&'static str, fn(Context) -> Context);

#[test]
fn snapshot_metadata_and_environment_mismatch_never_changes_the_lab() {
    let mut lab = support::counter();
    lab.native(|c| counter::increment_by(c, BoundedUint::new(2)?))
        .unwrap();
    let before = lab.snapshot();
    for field in [
        "version",
        "source",
        "generated artifact",
        "runtime ABI",
        "ledger version",
        "state mode",
    ] {
        let mut foreign = before.clone();
        match field {
            "version" => foreign.metadata.version += 1,
            "source" => foreign.metadata.artifacts.source_sha256[0] ^= 1,
            "generated artifact" => foreign.metadata.artifacts.generated_sha256[0] ^= 1,
            "runtime ABI" => foreign.metadata.runtime_abi += 1,
            "ledger version" => foreign.metadata.ledger_version.push('x'),
            _ => foreign.metadata.state_mode = "proof-ready".into(),
        }
        assert!(
            matches!(lab.restore(&foreign), Err(LabError::SnapshotMismatch(got)) if got == field)
        );
        assert_eq!(lab.snapshot().public_state(), before.public_state());
    }
    let mut env = support::environment();
    env.fixture_seed[0] ^= 1;
    let foreign = ContractLab::from_constructor(
        support::identity(),
        env,
        counter::initial_state(ConstructorContext::new(vec![])).unwrap(),
    )
    .unwrap();
    assert!(matches!(
        lab.restore(&foreign.snapshot()),
        Err(LabError::SnapshotMismatch("environment"))
    ));
    assert_eq!(lab.snapshot().public_state(), before.public_state());
}

#[test]
fn constructor_rejects_wallet_and_even_empty_nondefault_intent_plans() {
    for plan in [false, true] {
        let initial =
            counter::initial_state(ConstructorContext::new(Vec::<String>::new())).unwrap();
        let mut context = initial.into_circuit_context(ContractAddress::default());
        if plan {
            context.set_zswap_output_start(7).unwrap();
        } else {
            context.zswap_state.first_free = 1;
        }
        assert!(matches!(
            ContractLab::from_constructor(
                support::identity(),
                support::environment(),
                context.into_constructor_result()
            ),
            Err(LabError::UnsupportedState)
        ));
    }
}

#[test]
fn native_and_recorded_results_cannot_replace_environment_or_admit_zswap() {
    let mutations: &[Mutation] = &[
        ("gas", |mut c| {
            c.gas_limit = Some(RunningCost::ZERO);
            c
        }),
        ("cost", |mut c| {
            c.cost_model.noop_constant =
                CostDuration::from_picoseconds(c.cost_model.noop_constant.into_picoseconds() + 1);
            c
        }),
        ("identity", |c| c.with_coin_public_key_bytes([5; 32])),
        ("query address", |mut c| {
            c.query.address = ContractAddress(HashOutput([5; 32]));
            c
        }),
        ("own address", |mut c| {
            c.query.call_context.own_address = ContractAddress(HashOutput([5; 32]));
            c
        }),
        ("clock", |mut c| {
            c.query.call_context.tblock = Timestamp::from_secs(9);
            c
        }),
        ("uncertainty", |mut c| {
            c.query.call_context.tblock_err += 1;
            c
        }),
        ("parent", |mut c| {
            c.query.call_context.parent_block_hash = HashOutput([5; 32]);
            c
        }),
        ("last block", |mut c| {
            c.query.call_context.last_block_time = Timestamp::from_secs(9);
            c
        }),
        ("caller", |mut c| {
            c.query.call_context.caller = Some(PublicAddress::Contract(ContractAddress(
                HashOutput([5; 32]),
            )));
            c
        }),
        ("balance", |mut c| {
            c.query.call_context.balance = c.query.call_context.balance.insert(TokenType::Dust, 1);
            c
        }),
        ("indices", |mut c| {
            c.query.call_context.com_indices = c
                .query
                .call_context
                .com_indices
                .insert(CoinCommitment(HashOutput([5; 32])), 3);
            c
        }),
        ("wallet", |mut c| {
            c.zswap_state.first_free = 1;
            c
        }),
        ("plan", |mut c| {
            c.set_zswap_output_start(7).unwrap();
            c
        }),
    ];
    for (name, mutate) in mutations {
        let mut lab = support::counter();
        let before = lab.snapshot();
        let n = lab.native(|mut c| {
            c.private_state.push("secret not committed".into());
            Ok(CircuitFrame::new(mutate(c)).finish(()))
        });
        let r = lab.recorded(|mut c| {
            c.private_state.push("secret not committed".into());
            let mut r = RecordingFrame::new(c).finish(());
            r.execution.context = mutate(r.execution.context);
            Ok(r)
        });
        if *name == "wallet" || *name == "plan" {
            assert!(matches!(n, Err(LabError::UnsupportedState)), "{name}");
            assert!(matches!(r, Err(LabError::UnsupportedState)), "{name}");
        } else {
            assert!(matches!(n, Err(LabError::EnvironmentChanged)), "{name}");
            assert!(matches!(r, Err(LabError::EnvironmentChanged)), "{name}");
        }
        assert_eq!(lab.snapshot().public_state(), before.public_state());
        assert!(lab.private_state().is_empty());
    }
}

#[test]
fn recording_must_start_at_supplied_state_and_keep_sealed_identity() {
    let mut lab = support::counter();
    let initial = lab.snapshot();
    let error = lab
        .recorded(|c| {
            let c = counter::increment_by(c, BoundedUint::new(2)?)?.context;
            Ok(RecordingFrame::new(c).finish(()))
        })
        .unwrap_err();
    assert!(matches!(error, LabError::InitialContextMismatch));
    let error = lab
        .recorded(|c| {
            let original = c.into_constructor_result();
            let copy = midnight_compact_testkit::runtime::context::ConstructorResult::new(
                ConstructorContext::new(original.private_state.clone()),
                original.ledger_state.clone(),
            );
            let c = copy
                .into_circuit_context(ContractAddress::default())
                .with_coin_public_key_bytes([9; 32]);
            let mut r = RecordingFrame::new(c).finish(());
            r.execution.context = original.into_circuit_context(ContractAddress::default());
            Ok(r)
        })
        .unwrap_err();
    assert!(matches!(error, LabError::IdentityMismatch));
    assert_eq!(lab.snapshot().public_state(), initial.public_state());
}

#[test]
fn replay_state_effects_and_error_precedence_do_not_commit_tampered_execution() {
    for (state, effects) in [(true, false), (false, true), (true, true)] {
        let mut lab = support::counter();
        let before = lab.snapshot();
        let error = lab
            .recorded(|mut c| {
                let initial = c.query.state.clone();
                c.private_state.push("discard me".into());
                let mut r = counter::recorded::increment_by(c, BoundedUint::new(2)?)?;
                if state {
                    r.execution.context.query.state = initial;
                }
                if effects {
                    let e = &mut r.execution.context.query.effects;
                    e.shielded_mints = e.shielded_mints.insert(HashOutput([17; 32]), 1);
                }
                Ok(r)
            })
            .unwrap_err();
        if effects {
            assert!(matches!(error, LabError::ReplayEffectsMismatch));
        } else {
            assert!(matches!(error, LabError::ReplayStateMismatch));
        }
        assert_eq!(lab.snapshot().public_state(), before.public_state());
        assert!(lab.private_state().is_empty());
    }
}

#[test]
fn local_empty_replay_is_distinct_from_native_and_effects_reset_each_call() {
    let mut lab = support::counter();
    let r = lab
        .recorded(|c| Ok(RecordingFrame::new(c).finish(7)))
        .unwrap();
    assert_eq!(*r.output(), 7);
    assert!(r.replay().unwrap().program().is_empty());
    let first = lab
        .native(|mut c| {
            c.query.effects.shielded_mints = c
                .query
                .effects
                .shielded_mints
                .insert(HashOutput([17; 32]), 1);
            Ok(CircuitFrame::new(c).finish(()))
        })
        .unwrap();
    assert!(!first.effects().shielded_mints.is_empty());
    let second = lab
        .recorded(|c| {
            assert!(c.query.effects.shielded_mints.is_empty());
            Ok(RecordingFrame::new(c).finish(()))
        })
        .unwrap();
    assert!(second.effects().shielded_mints.is_empty());
}

#[test]
fn script_mismatch_exhaustion_error_and_rollback_preserve_owned_checkpoint() {
    let secret = "private script sentinel";
    let mut script = WitnessScript::new([
        (1, Ok(secret.to_owned())),
        (2, Err(CompactError::AssertionFailed(secret.into()))),
    ]);
    assert_eq!(
        script.answer(9),
        Err(CompactError::AssertionFailed(
            "witness arguments differ from script".into()
        ))
    );
    assert_eq!(script.remaining(), 2);
    assert!(script.journal().is_empty());
    assert_eq!(script.answer(1).unwrap(), secret);
    assert_eq!(
        script.answer(2),
        Err(CompactError::AssertionFailed(secret.into()))
    );
    assert_eq!(script.remaining(), 0);
    assert_eq!(script.journal(), &[1, 2]);
    assert_eq!(
        script.answer(2),
        Err(CompactError::AssertionFailed(
            "witness script exhausted".into()
        ))
    );
    assert_eq!(script.remaining(), 0);
    assert_eq!(script.journal(), &[1, 2]);
    assert!(!format!("{script:?}").contains(secret));
    let initial =
        counter::initial_state(ConstructorContext::new(WitnessScript::new([(1, Ok(4))]))).unwrap();
    let mut lab =
        ContractLab::from_constructor(support::identity(), support::environment(), initial)
            .unwrap();
    let result = lab.native::<()>(|mut c| {
        assert_eq!(c.private_state.answer(1)?, 4);
        Err(CompactError::AssertionFailed(secret.into()))
    });
    assert!(result.is_err());
    assert_eq!(lab.private_state().remaining(), 1);
    assert!(lab.private_state().journal().is_empty());
}

#[test]
fn diagnostics_redact_outputs_private_state_transcripts_and_underlying_errors() {
    let secret = "SECRET_PAYLOAD_248";
    let mut lab = support::counter();
    let report = lab
        .native(|mut c| {
            c.private_state.push(secret.into());
            let mut r = CircuitFrame::new(c).finish(secret.to_owned());
            r.private_transcript_outputs.push(
                midnight_compact_testkit::runtime::fab::AlignedValue::from(true),
            );
            Ok(r)
        })
        .unwrap();
    assert!(!format!("{report:?} {:?}", lab.snapshot()).contains(secret));
    assert_eq!(report.output(), secret);
    assert_eq!(lab.private_state(), &[secret.to_owned()]);
    assert_eq!(lab.snapshot().private_state(), &[secret.to_owned()]);
    for error in [
        LabError::Execution(CompactError::AssertionFailed(secret.into())),
        LabError::Replay(secret.into()),
    ] {
        assert!(!format!("{error:?}: {error}").contains(secret));
        assert!(std::error::Error::source(&error).is_none());
        match &error {
            LabError::Execution(expected) => {
                assert_eq!(error.execution_error(), Some(expected));
                assert!(error.replay_error().is_none());
            }
            LabError::Replay(expected) => {
                assert_eq!(error.replay_error(), Some(expected.as_str()));
                assert!(error.execution_error().is_none());
            }
            _ => unreachable!(),
        }
    }
    let failures = [
        LabError::SnapshotMismatch("version"),
        LabError::UnsupportedState,
        LabError::EnvironmentChanged,
        LabError::InitialContextMismatch,
        LabError::IdentityMismatch,
        LabError::ReplayStateMismatch,
        LabError::ReplayEffectsMismatch,
    ];
    let messages: std::collections::BTreeSet<_> = failures.iter().map(|e| e.to_string()).collect();
    assert_eq!(
        messages.len(),
        failures.len(),
        "public failure reasons remain distinguishable"
    );
    for error in failures {
        assert!(error.execution_error().is_none());
        assert!(error.replay_error().is_none());
        assert!(std::error::Error::source(&error).is_none());
    }
}

#[test]
fn explicit_clock_identity_and_seed_are_retained_across_valid_calls_and_restore() {
    let mut env = support::environment();
    env.address = ContractAddress(HashOutput([24; 32]));
    env.block.tblock = Timestamp::from_secs(1234);
    env.block.last_block_time = Timestamp::from_secs(1230);
    env.block.tblock_err = 2;
    env.coin_public_key = Some([48; 32]);
    let initial = counter::initial_state(ConstructorContext::new(())).unwrap();
    let mut lab = ContractLab::from_constructor(support::identity(), env, initial).unwrap();
    let checkpoint = lab.snapshot();
    let r = lab
        .recorded(|c| {
            assert_eq!(c.own_coin_public_key()?, [48; 32]);
            assert_eq!(c.query.call_context.tblock.to_secs(), 1234);
            counter::recorded::increment_by(c, BoundedUint::new(2)?)
        })
        .unwrap();
    assert!(!r.replay().unwrap().program().is_empty());
    lab.restore(&checkpoint).unwrap();
    assert_eq!(lab.environment().coin_public_key, Some([48; 32]));
    assert_eq!(lab.environment().fixture_seed, [7; 32]);
    assert_eq!(lab.snapshot().public_state(), checkpoint.public_state());
}

#[test]
fn trusted_adapter_boundary_does_not_attest_transient_policy_changes() {
    let mut env = support::environment();
    env.query_gas_limit = Some(RunningCost::ZERO);
    let initial = counter::initial_state(ConstructorContext::new(())).unwrap();
    let mut lab = ContractLab::from_constructor(support::identity(), env, initial).unwrap();
    let before = lab.snapshot();
    // The direct generated adapter honors the configured per-query limit.
    assert!(matches!(
        lab.recorded(|c| counter::recorded::increment_by(c, BoundedUint::new(1)?)),
        Err(LabError::Execution(CompactError::LedgerQueryRejected(_)))
    ));
    assert_eq!(lab.snapshot().public_state(), before.public_state());
    // Deliberately dishonest arbitrary Rust can clear then restore that policy.
    // This accepted result documents the sandbox limit; it is NOT a supported
    // adapter pattern or a claim that the configured budget was respected.
    let report = lab
        .recorded(|mut c| {
            let saved_limit = c.gas_limit;
            c.gas_limit = None;
            let mut recorded = counter::recorded::increment_by(c, BoundedUint::new(1)?)?;
            recorded.execution.context.gas_limit = saved_limit;
            Ok(recorded)
        })
        .unwrap();
    assert_ne!(report.public_state(), before.public_state());
    assert_ne!(report.replay().unwrap().gas(), RunningCost::ZERO);
    assert_eq!(lab.environment().query_gas_limit, Some(RunningCost::ZERO));
}

#[test]
fn script_failures_after_public_write_roll_back_and_require_explicit_inspection() {
    for (answers, expected) in [
        (
            vec![(1, Ok(4)), (3, Ok(5))],
            "witness arguments differ from script",
        ),
        (vec![(1, Ok(4))], "witness script exhausted"),
        (
            vec![
                (1, Ok(4)),
                (
                    2,
                    Err(CompactError::AssertionFailed("PRIVATE_SCRIPT_ERROR".into())),
                ),
            ],
            "PRIVATE_SCRIPT_ERROR",
        ),
    ] {
        for recorded in [false, true] {
            let count = answers.len();
            let initial = counter::initial_state(ConstructorContext::new(WitnessScript::new(
                answers.clone(),
            )))
            .unwrap();
            let mut lab =
                ContractLab::from_constructor(support::identity(), support::environment(), initial)
                    .unwrap();
            let before = lab.snapshot();
            let error = if recorded {
                lab.recorded::<()>(|c| {
                    let mut changed = counter::recorded::increment_by(c, BoundedUint::new(2)?)?;
                    assert_ne!(
                        changed.execution.context.query.state,
                        *before.public_state()
                    );
                    assert_eq!(changed.execution.context.private_state.answer(1)?, 4);
                    changed.execution.context.private_state.answer(2)?;
                    panic!("script must fail")
                })
                .unwrap_err()
            } else {
                lab.native::<()>(|c| {
                    let mut changed = counter::increment_by(c, BoundedUint::new(2)?)?;
                    assert_ne!(changed.context.query.state, *before.public_state());
                    assert_eq!(changed.context.private_state.answer(1)?, 4);
                    changed.context.private_state.answer(2)?;
                    panic!("script must fail")
                })
                .unwrap_err()
            };
            assert!(matches!(error, LabError::Execution(_)));
            assert_eq!(
                error.execution_error(),
                Some(&CompactError::AssertionFailed(expected.into()))
            );
            assert_eq!(
                format!("{error}"),
                "circuit execution failed (details redacted)"
            );
            assert_eq!(format!("{error:?}"), error.to_string());
            assert!(std::error::Error::source(&error).is_none());
            assert_eq!(lab.snapshot().public_state(), before.public_state());
            assert_eq!(lab.private_state().remaining(), count);
            assert!(lab.private_state().journal().is_empty());
        }
    }
}

#[test]
fn application_assertions_with_script_text_are_generic_redacted_execution_errors() {
    for text in [
        "witness script exhausted",
        "witness arguments differ from script",
    ] {
        let mut lab = support::counter();
        let before = lab.snapshot();
        let error = lab
            .native::<()>(|mut c| {
                c.private_state.push("PRIVATE_APPLICATION_STATE".into());
                Err(CompactError::AssertionFailed(text.into()))
            })
            .unwrap_err();
        assert!(matches!(error, LabError::Execution(_)));
        assert_eq!(
            error.execution_error(),
            Some(&CompactError::AssertionFailed(text.into()))
        );
        assert_eq!(
            format!("{error:?}"),
            "circuit execution failed (details redacted)"
        );
        assert_eq!(error.to_string(), format!("{error:?}"));
        assert!(std::error::Error::source(&error).is_none());
        assert_eq!(lab.snapshot().public_state(), before.public_state());
        assert!(lab.private_state().is_empty());
    }
    let mut script = WitnessScript::new([("PRIVATE_ARGUMENT", Ok("PRIVATE_ANSWER"))]);
    assert!(!format!("{script:?}").contains("PRIVATE_"));
    assert_eq!(script.answer("PRIVATE_ARGUMENT"), Ok("PRIVATE_ANSWER"));
    assert_eq!(script.journal(), &["PRIVATE_ARGUMENT"]);
    assert!(!format!("{script:?}").contains("PRIVATE_"));
}
