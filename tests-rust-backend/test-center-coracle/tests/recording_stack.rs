// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Process-isolated stack regression and opt-in allocation measurements.
//! The counting allocator is test instrumentation, never runtime product code.
#[path = "../support/withdraw.rs"]
mod support;
use compact_rust_test_center_coracle_fixture::{ledger_contract as c, types};
use midnight_compact_runtime as runtime;
use runtime::context::CircuitResult;
use runtime::recording::{PublicTrace, RecordedCircuitResult, RecordingFrame};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::hint::black_box;
use std::mem::size_of;
use std::process::Command;

#[derive(Clone, Copy, Default, serde::Serialize)]
struct Counts {
    allocations: usize,
    deallocations: usize,
    allocated_bytes: usize,
    freed_bytes: usize,
}
thread_local! {
    static COUNTS: Cell<Option<Counts>> = const { Cell::new(None) };
}
fn count(allocated: usize, freed: usize) {
    let _ = COUNTS.try_with(|cell| {
        if let Some(mut c) = cell.get() {
            c.allocations += usize::from(allocated != 0);
            c.deallocations += usize::from(freed != 0);
            c.allocated_bytes += allocated;
            c.freed_bytes += freed;
            cell.set(Some(c));
        }
    });
}
struct CountedSystem;
// SAFETY: every operation forwards the caller's unchanged allocation contract
// to System. Thread-local counters neither allocate nor expose allocations.
unsafe impl GlobalAlloc for CountedSystem {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the caller supplies a valid layout for System.
        let p = unsafe { System.alloc(layout) };
        if !p.is_null() {
            count(layout.size(), 0);
        }
        p
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the caller supplies a valid layout for System.
        let p = unsafe { System.alloc_zeroed(layout) };
        if !p.is_null() {
            count(layout.size(), 0);
        }
        p
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        count(0, layout.size());
        // SAFETY: ptr and layout are forwarded unchanged to their allocator.
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        // SAFETY: the caller's original pointer/layout and requested size are
        // forwarded unchanged. Failed realloc leaves the original live.
        let p = unsafe { System.realloc(ptr, layout, size) };
        if !p.is_null() {
            count(size, layout.size());
        }
        p
    }
}
#[global_allocator]
static ALLOCATOR: CountedSystem = CountedSystem;

fn measure(name: &str, f: impl FnOnce()) {
    COUNTS.with(|cell| cell.set(Some(Counts::default())));
    f();
    let count = COUNTS.with(|cell| cell.replace(None).unwrap());
    println!(
        "allocation {name}: {}",
        serde_json::to_string(&count).unwrap()
    );
}
fn settings(red: bool) -> support::Settings {
    support::Settings {
        red,
        phase: if red {
            types::State::red_wins
        } else {
            types::State::blue_wins
        },
        ..Default::default()
    }
}
#[inline(never)]
fn withdraw(red: bool) {
    let s = settings(red);
    let context = support::seeded(&s).unwrap();
    let witness = support::Witness::new(s);
    measure(if red { "red" } else { "blue" }, || {
        let result = c::recorded::withdraw(context, &witness).unwrap();
        assert_eq!(result.execution.private_transcript_outputs.len(), 8);
        assert_eq!(result.execution.context.private_state.calls, 2);
        drop(black_box(result));
    });
}
fn metrics() {
    let context = support::seeded(&settings(true)).unwrap();
    println!(
        "sizes: {}",
        serde_json::json!({
            "frame": size_of::<RecordingFrame<support::Private>>(),
            "context": size_of::<runtime::context::CircuitContext<support::Private>>(),
            "query": size_of::<runtime::ledger::QueryContext<runtime::ledger::DefaultDB>>(),
            "plan": size_of::<runtime::CircuitZswapPlan>(),
            "cost_model": std::mem::size_of_val(&context.cost_model),
            "trace": size_of::<PublicTrace>(),
            "recorded_result": size_of::<RecordedCircuitResult<support::Private,types::WithdrawnCoins>>(),
            "qualified_coin": size_of::<types::QualifiedShieldedCoinInfo>(),
            "withdrawn_coins": size_of::<types::WithdrawnCoins>(),
        })
    );
    // Context setup is outside counting; inherited allocations are freed inside
    // the interval. Compare deltas, not absolute net ownership/leak claims.
    measure("new_finish", || {
        drop(black_box(RecordingFrame::new(context).finish(())))
    });
    let context = support::seeded(&settings(true)).unwrap();
    measure("local_success", || {
        let (frame, ()) = RecordingFrame::new(context)
            .call_local(|context| {
                Ok(CircuitResult {
                    context,
                    result: (),
                    gas_cost: runtime::context::RunningCost::ZERO,
                    private_transcript_outputs: vec![],
                })
            })
            .unwrap();
        drop(black_box(frame.finish(())));
    });
    let context = support::seeded(&settings(true)).unwrap();
    measure("local_error", || {
        let result = RecordingFrame::new(context)
            .call_local::<(), _>(|_| Err(runtime::CompactError::MissingCoinPublicKey));
        assert!(matches!(
            result,
            Err(runtime::CompactError::MissingCoinPublicKey)
        ));
    });
    let context = support::seeded(&settings(true)).unwrap();
    measure("witness_error", || {
        let result = RecordingFrame::new(context)
            .try_witness_metered::<(), _>(|_, _| Err(runtime::CompactError::MissingCoinPublicKey));
        assert!(matches!(
            result,
            Err(runtime::CompactError::MissingCoinPublicKey)
        ));
    });
    withdraw(true);
    withdraw(false);
}
#[test]
fn stack_child() {
    let Ok(case) = std::env::var("COMPACT_STACK_CASE") else {
        return;
    };
    if case == "metrics" {
        // Measurement baseline must complete even before the stack fix.
        std::thread::Builder::new()
            .stack_size(8 * 1024 * 1024)
            .spawn(metrics)
            .unwrap()
            .join()
            .unwrap();
    } else if std::env::var_os("COMPACT_STACK_EXPLICIT").is_some() {
        std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(move || withdraw(case == "red"))
            .unwrap()
            .join()
            .unwrap();
    } else {
        // This test itself runs on the unconfigured normal libtest worker.
        withdraw(case == "red");
    }
}
#[test]
fn original_withdraw_fits_normal_and_explicit_two_mib_workers() {
    for red in [true, false] {
        for explicit in [false, true] {
            let mut command = Command::new(std::env::current_exe().unwrap());
            command
                .args(["--exact", "stack_child", "--nocapture"])
                .env_remove("RUST_MIN_STACK")
                .env_remove("COMPACT_STACK_EXPLICIT")
                .env("COMPACT_STACK_CASE", if red { "red" } else { "blue" });
            if explicit {
                command.env("COMPACT_STACK_EXPLICIT", "1");
            }
            let output = command.output().unwrap();
            assert!(
                output.status.success(),
                "red={red}, explicit={explicit}: {}\n{}\n{}",
                output.status,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
}
