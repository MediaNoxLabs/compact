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
// Executing width gate for `mbits->rust-width` / `arith-binop-rust`.
//
// The emitter casts both operands of unsigned `+`/`-`/`*` to the width
// implied by the typer's `mbits`, chosen from a five-rung ladder by `<=`
// boundary comparisons. Byte-parity pins the generated TEXT, but a wrong
// rung produces text that is still perfectly valid Rust — so the corpus
// alone cannot distinguish `as u32` from `as u16` in a multiplication
// that overflows u16.
//
// It matters because the operation is `wrapping_*`: at too narrow a width
// it does not panic, it silently truncates. Every case below is chosen so
// that a one-rung-too-narrow cast yields a specific WRONG VALUE rather
// than an error, and each test states that value. These are the
// assertions that would actually fail if a ladder boundary regressed.

use compact_contract_widening_arith_fixture::pure_circuits;

/// mbits = 9 -> u16, widening from u8 operands.
///
/// `255 + 255 = 510`, which does not fit in u8. A cast left at the
/// operands' native u8 width would wrap to `254`.
#[test]
fn u8_addition_widens_to_u16() {
    assert_eq!(pure_circuits::sum_bytes(255, 255).unwrap(), 510);
}

/// mbits = 17 -> u32, widening from a u8 operand times a literal.
///
/// This is the `Uint<8> * 365` shape the emitter's own comment cites, and
/// the one the removed `digital-passport` fixture was the sole carrier of.
/// `255 * 365 = 93_075` exceeds u16, so a u16 rung would wrap to `27_539`
/// and a u8 rung to `19`.
#[test]
fn u8_times_literal_widens_to_u32() {
    assert_eq!(pure_circuits::age_threshold_days(255).unwrap(), 93_075);
}

/// mbits = 16 -> u16. Pins the u16/u32 boundary from BELOW: 16 must still
/// map to u16, so this guards against a boundary loosened to `< 16`.
///
/// `255 * 255 = 65_025` is the largest value a u8*u8 product can take and
/// still fits u16 exactly; at a u8 rung it would wrap to `1`.
#[test]
fn u8_product_stays_at_u16_upper_edge() {
    assert_eq!(pure_circuits::product_bytes(255, 255).unwrap(), 65_025);
}

/// mbits = 32 -> u32, widening from u16 rather than u8 (so the widening
/// path is exercised from a second source width). Pins the u32/u64
/// boundary from below.
///
/// `65_535 * 65_535 = 4_294_836_225` fits u32 exactly; at a u16 rung it
/// would wrap to `1`.
#[test]
fn u16_product_widens_to_u32_upper_edge() {
    assert_eq!(
        pure_circuits::area_of(65_535, 65_535).unwrap(),
        4_294_836_225
    );
}

/// The no-op direction: when operands are already the result width the
/// cast must not change the value. `2 * 3` at any rung is 6, so this is a
/// sanity anchor rather than a boundary test.
#[test]
fn small_values_are_unaffected_by_the_cast() {
    assert_eq!(pure_circuits::sum_bytes(2, 3).unwrap(), 5);
    assert_eq!(pure_circuits::product_bytes(2, 3).unwrap(), 6);
    assert_eq!(pure_circuits::area_of(2, 3).unwrap(), 6);
    assert_eq!(pure_circuits::age_threshold_days(1).unwrap(), 365);
}
