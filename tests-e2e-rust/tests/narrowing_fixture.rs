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

//
// Narrowing casts: an executing test, deliberately not a snapshot.
//
// The TypeScript backend lowers `x as Uint<N>` to a bounds check that
// throws; the Rust backend used to lower it to Rust's `as`, which checks
// nothing (MediaNoxLabs/compact#51). Byte parity is structurally blind to
// that: it compares committed Rust against regenerated Rust, and
// `(x) as u8` agrees with itself perfectly. The disagreement only exists at
// run time, for inputs no fixture fed it.
//
// So these assertions run the generated code. Each one names the value that
// used to come back wrong, because "returns Err" is a much weaker claim than
// "does not return 44 where TypeScript throws".

use compact_contract_narrowing_fixture::pure_circuits;
use compact_runtime::CompactError;

/// Values inside the declared bound are returned unchanged. This is the
/// assertion that keeps the fix from being "reject everything".
#[test]
fn in_range_values_pass_through() {
    for x in [0u16, 1, 50, 98, 99] {
        assert_eq!(
            pure_circuits::shrink(x).expect("in-range value must be accepted"),
            x as u8,
            "shrink({x}) should be {x}"
        );
    }
}

/// `Uint<0..100>` admits 0..=99, so 99 is the last accepted value and 100
/// the first rejected one. Compact's range is exclusive of its upper bound,
/// and getting that off by one is precisely what a hand-written cast does.
#[test]
fn the_boundary_is_the_compact_bound_not_the_rust_one() {
    assert_eq!(pure_circuits::shrink(99).unwrap(), 99);
    assert!(
        pure_circuits::shrink(100).is_err(),
        "100 is outside Uint<0..100> and must be rejected"
    );
}

/// The quiet failure mode. 100..=255 fits `u8`, so Rust's `as` returned it
/// unchanged: a value outside the declared Compact type, not truncated, with
/// nothing to make it visible. TypeScript throws for all of these.
#[test]
fn values_inside_u8_but_outside_the_compact_type_are_rejected() {
    for x in [100u16, 101, 150, 200, 255] {
        let r = pure_circuits::shrink(x);
        assert!(
            r.is_err(),
            "shrink({x}) returned Ok({:?}); TypeScript throws here, and `as` \
             used to hand this straight back as an out-of-range value",
            r.ok()
        );
    }
}

/// The corrupting failure mode. `300 as u8` is 44 — an out-of-range input
/// became a plausible in-range output, which is worse than an error and
/// worse than a crash.
#[test]
fn out_of_range_values_are_not_truncated_into_plausible_ones() {
    let r = pure_circuits::shrink(300);
    assert!(
        r.is_err(),
        "shrink(300) returned Ok({:?}) — `300 as u8` is 44, and 44 is a \
         perfectly believable Uint<0..100>",
        r.ok()
    );

    // Spell the specific corruption out, so a regression that reintroduces
    // `as` fails on the value rather than only on the Result shape.
    assert_ne!(
        pure_circuits::shrink(300).unwrap_or(0),
        44,
        "shrink(300) must never be 44"
    );
}

/// The failure carries the same text TypeScript throws, so a contract that
/// runs on both backends reports one story rather than two.
#[test]
fn the_error_reproduces_the_typescript_message() {
    let err = pure_circuits::shrink(300).unwrap_err();
    let msg = err.to_string();

    assert!(
        matches!(err, CompactError::CastFailed(_)),
        "a failed cast is not an assertion failure; it should not be reported \
         as one, got: {msg}"
    );
    assert!(
        msg.contains("cast from Field or Uint value to smaller Uint value failed"),
        "message should match the TypeScript wording, got: {msg}"
    );
    assert!(
        msg.contains("300 is greater than 99"),
        "message should name the value and the Compact bound, got: {msg}"
    );
    assert!(
        msg.starts_with("narrowing_fixture.compact line "),
        "message should point at the Compact source, not at generated Rust, \
         got: {msg}"
    );
}

/// The same two failure modes at a wider pair (u32 source, u16 target), so a
/// regression cannot hide behind everything fitting in one byte.
#[test]
fn wider_casts_behave_the_same_way() {
    assert_eq!(pure_circuits::shrink_wide(65_534).unwrap(), 65_534);
    assert!(
        pure_circuits::shrink_wide(65_535).is_err(),
        "Uint<0..65535> admits up to 65534; 65535 is out of range"
    );
    assert!(
        pure_circuits::shrink_wide(70_000).is_err(),
        "70000 must not wrap to 4464"
    );
    assert_ne!(
        pure_circuits::shrink_wide(70_000).unwrap_or(0),
        4_464,
        "70000 as u16 is 4464; that must not be the answer"
    );
}
