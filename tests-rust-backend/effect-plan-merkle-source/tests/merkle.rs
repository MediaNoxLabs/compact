// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use compact_rust_effect_plan_merkle_source_fixture::{
    ledger_contract as c, ledger_slots as slots, runtime as r, support as reference,
};
use r::context::CircuitResult;
use r::recording::RecordedCircuitResult;
use reference::{Context, Index, Leaf, context, root};

fn equivalent<T: std::fmt::Debug + PartialEq>(
    native: CircuitResult<u64, T>,
    recorded: RecordedCircuitResult<u64, T>,
    manual: RecordedCircuitResult<u64, T>,
) -> (Context, Context, Context) {
    for execution in [&recorded.execution, &manual.execution] {
        assert_eq!(execution.result, native.result);
        assert_eq!(execution.context.query.state, native.context.query.state);
        assert_eq!(
            execution.context.query.effects,
            native.context.query.effects
        );
        assert_eq!(execution.gas_cost, native.gas_cost);
        assert_eq!(execution.context.private_state, 7);
        assert_eq!(
            execution.private_transcript_outputs,
            native.private_transcript_outputs
        );
    }
    assert_eq!(
        serde_json::to_value(recorded.public.verify_ops()).unwrap(),
        serde_json::to_value(manual.public.verify_ops()).unwrap()
    );
    let replay = recorded
        .public
        .initial()
        .query(
            recorded.public.verify_ops(),
            None,
            &recorded.execution.context.cost_model,
        )
        .unwrap();
    let reference = manual
        .public
        .initial()
        .query(
            manual.public.verify_ops(),
            None,
            &manual.execution.context.cost_model,
        )
        .unwrap();
    assert_eq!(replay.context.state, native.context.query.state);
    assert_eq!(replay.context.effects, native.context.query.effects);
    assert_eq!(replay.gas_cost, reference.gas_cost);
    (
        native.context,
        recorded.execution.context,
        manual.execution.context,
    )
}
macro_rules! step {
    ($contexts:ident, $name:ident $(, $argument:expr)* $(,)?) => {{
        let (a, b, m) = $contexts;
        $contexts = equivalent(c::$name(a $(, $argument)*).unwrap(), c::recorded::$name(b $(, $argument)*).unwrap(), reference::$name(m $(, $argument)*).unwrap());
    }};
}
#[test]
fn named_helpers_and_selected_append_preserve_root_history_and_replay() {
    for selected in [false, true] {
        for n in [0, 1, 255] {
            let mut contexts = (context(), context(), context());
            let empty = root(&contexts.0);
            let leaf = Leaf::new(n).unwrap();
            step!(contexts, run, leaf, empty.clone(), selected);
            let state = contexts.0.query.state.get_ref();
            assert_eq!(
                slots::plain
                    .inspect(state)
                    .unwrap()
                    .first_free()
                    .unwrap()
                    .value(),
                1
            );
            assert_eq!(
                slots::historic
                    .inspect(state)
                    .unwrap()
                    .first_free()
                    .unwrap()
                    .value(),
                u128::from(selected)
            );
            let historical = c::observe(contexts.0, empty.clone()).unwrap();
            assert!(historical.result.historic_known);
            assert!(!historical.result.plain_full);
            assert!(!historical.result.historic_full);
            contexts = equivalent(
                historical,
                c::recorded::observe(contexts.1, empty.clone()).unwrap(),
                reference::observe(contexts.2, empty).unwrap(),
            );
            let current = root(&contexts.0);
            step!(contexts, observe, current.clone());
            assert_eq!(contexts.0.private_state, 7);
        }
    }
}
#[test]
fn indexed_hash_default_and_reset_operations_compose_on_reused_contexts() {
    let mut contexts = (context(), context(), context());
    let blank = root(&contexts.0);
    step!(
        contexts,
        place,
        Leaf::new(37).unwrap(),
        Index::new(2).unwrap()
    );
    let old = root(&contexts.0);
    step!(
        contexts,
        hashes,
        r::FixedBytes::new([17; 32]),
        Index::new(5).unwrap()
    );
    step!(contexts, defaults, Index::new(7).unwrap());
    let latest = root(&contexts.0);
    let observed = c::observe(contexts.0, latest.clone()).unwrap();
    assert!(observed.result.plain_full && observed.result.historic_full);
    assert!(observed.result.plain_known && observed.result.historic_known);
    contexts = equivalent(
        observed,
        c::recorded::observe(contexts.1, latest.clone()).unwrap(),
        reference::observe(contexts.2, latest.clone()).unwrap(),
    );
    step!(contexts, observe, old.clone());
    assert!(
        slots::historic
            .inspect(contexts.0.query.state.get_ref())
            .unwrap()
            .contains_root(r::ledger::MerkleTreeDigest(old.field))
    );
    step!(contexts, forget);
    let historic = slots::historic
        .inspect(contexts.0.query.state.get_ref())
        .unwrap();
    assert!(!historic.contains_root(r::ledger::MerkleTreeDigest(old.field)));
    assert!(historic.contains_root(r::ledger::MerkleTreeDigest(latest.field)));
    step!(contexts, reset);
    assert_eq!(root(&contexts.0), blank);
    let historic = slots::historic
        .inspect(contexts.0.query.state.get_ref())
        .unwrap();
    assert_eq!(historic.first_free().unwrap().value(), 0);
    assert!(!historic.contains_root(r::ledger::MerkleTreeDigest(latest.field)));
    assert!(historic.contains_root(r::ledger::MerkleTreeDigest(blank.field)));
    step!(contexts, observe, blank.clone());
    assert_eq!(contexts.0.private_state, 7);
}
#[test]
fn invalid_indices_and_full_tree_append_refuse_consistently() {
    for index in [8, u64::MAX as u128] {
        let leaf = Leaf::new(9).unwrap();
        let index = Index::new(index).unwrap();
        let a = c::place(context(), leaf, index).err().unwrap();
        assert_eq!(a, c::recorded::place(context(), leaf, index).err().unwrap());
        assert_eq!(a, reference::place(context(), leaf, index).err().unwrap());
        let a = c::defaults(context(), index).err().unwrap();
        assert_eq!(a, c::recorded::defaults(context(), index).err().unwrap());
        assert_eq!(a, reference::defaults(context(), index).err().unwrap());
        let hash = r::FixedBytes::new([9; 32]);
        let a = c::hashes(context(), hash, index).err().unwrap();
        assert_eq!(
            a,
            c::recorded::hashes(context(), hash, index).err().unwrap()
        );
        assert_eq!(a, reference::hashes(context(), hash, index).err().unwrap());
    }
    let mut contexts = (context(), context(), context());
    step!(contexts, defaults, Index::new(7).unwrap());
    let root = root(&contexts.0);
    let leaf = Leaf::new(1).unwrap();
    let a = c::run(contexts.0, leaf, root.clone(), true).err().unwrap();
    assert_eq!(
        a,
        c::recorded::run(contexts.1, leaf, root.clone(), true)
            .err()
            .unwrap()
    );
    assert_eq!(
        a,
        reference::run(contexts.2, leaf, root, true).err().unwrap()
    );
}

#[test]
fn source_generated_nested_paths_record_distinct_tree_segments() {
    use compact_rust_effect_plan_merkle_source_fixture::nested::{
        ledger_contract as n, ledger_slots as ns,
    };
    use r::context::ConstructorContext;
    use r::recording::RecordingFrame;
    let context = || {
        n::initial_state(ConstructorContext::new(7_u64))
            .unwrap()
            .into_circuit_context(Default::default())
    };
    assert_eq!(ns::plain.path(), &[0, 0]);
    assert_eq!(ns::historic.path(), &[1, 14]);
    let mut contexts = (context(), context(), context());
    for n_value in [0_u64, 255, 37] {
        let leaf = r::Field::from(n_value);
        let frame = ns::plain
            .record_insert(RecordingFrame::new(contexts.2), leaf)
            .unwrap();
        let manual = ns::historic.record_insert(frame, leaf).unwrap().finish(());
        contexts = equivalent(
            n::run(contexts.0, leaf).unwrap(),
            n::recorded::run(contexts.1, leaf).unwrap(),
            manual,
        );
        let state = contexts.0.query.state.get_ref();
        assert_eq!(
            ns::plain.inspect(state).unwrap().root(),
            ns::historic.inspect(state).unwrap().root()
        );
    }
    assert_eq!(
        ns::plain
            .inspect(contexts.0.query.state.get_ref())
            .unwrap()
            .first_free()
            .unwrap()
            .value(),
        3
    );
    assert_eq!(
        ns::historic
            .inspect(contexts.0.query.state.get_ref())
            .unwrap()
            .first_free()
            .unwrap()
            .value(),
        3
    );
}
