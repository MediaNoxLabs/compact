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
// literal_coercion_fixture.compact executing gate.
//
// The fixture puts a bare literal (or a Uint value) in every position
// where the typechecker had to be honoured by the emitter — Field const
// RHS, struct Field member, `some<Field>(0)`, `persistentHash([0])`, a
// native argument, a pure-circuit argument, a return tail, a scalar
// Uint→Field, and an aggregate Field vector. Before the fix each such
// position emitted an un-typed Rust integer (`0`, ambiguous
// `AlignedValue::from(0)`, an `i32` where `Fr` was required) and the
// generated crate failed `cargo build`.
//
// Three assertions:
//   1. `initial_state()` serialised bytes match the TS reference — this
//      pins the destination-typed LEDGER writes (Field / Uint<64> /
//      `Vector<2, Field>` / `Bytes<32>` / `JubjubPoint`) including the
//      coerced hash and curve-point values.
//   2. each pure circuit returns the intended coerced value (the
//      round-trip), driven directly in Rust — the pure circuits have no
//      TS-side runtime entry point.

use compact_contract_literal_coercion_fixture::{ledger, pure_circuits, Contract, VecBox};
use midnight_compact_runtime::*;
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use tests_e2e_rust::SmallFixtureTsReference;

fn fixture() -> SmallFixtureTsReference {
    SmallFixtureTsReference::load(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fixtures/literal-coercion-fixture-ts-state.json"
    ))
}

fn ctor_ctx() -> ConstructorContext<()> {
    ConstructorContext {
        initial_private_state: (),
        empty_zswap_local_state: ZswapLocalState::default(),
        cost_model: INITIAL_COST_MODEL.clone(),
        gas_limit: None,
    }
}

/// The fixture has no impure circuits, so the operations map is empty; the
/// pure circuits live in `pure_circuits` and contribute nothing to the
/// on-chain state shape.
fn make_envelope(
    data: ChargedState<midnight_storage::DefaultDB>,
) -> ContractState<midnight_storage::DefaultDB> {
    ContractState {
        data,
        operations: HashMap::new(),
        maintenance_authority: ContractMaintenanceAuthority::default(),
        balance: Default::default(),
    }
}

/// The constructor seeds five destination-typed cells from literal / coerced
/// values; the serialised state must equal the TS backend's byte-for-byte.
#[test]
fn literal_coercion_init_byte_parity() {
    let ts_ref = fixture();
    let contract: Contract<(), NoWitnesses> = Contract::new(NoWitnesses);
    let result = contract.initial_state(ctor_ctx()).expect("initial_state");

    let envelope = make_envelope(result.current_contract_state.clone());
    let mut buf = Vec::new();
    tagged_serialize(&envelope, &mut buf).expect("tagged_serialize");

    let ts_bytes = ts_ref.after_init.state_bytes();
    assert_eq!(
        buf,
        ts_bytes,
        "Rust state bytes differ from TS reference\n\nRust ({} B): {}\n\nTS   ({} B): {}",
        buf.len(),
        hex::encode(&buf),
        ts_bytes.len(),
        hex::encode(&ts_bytes),
    );
}

/// Each ledger cell holds the value its coerced write produced — in
/// particular the `Uint<64>` cell commits a u64-aligned value, and the
/// `Vector<2, Field>` cell holds Field elements, not Bytes-aligned Uints.
#[test]
fn constructor_seeds_the_destination_typed_cells() {
    let contract: Contract<(), NoWitnesses> = Contract::new(NoWitnesses);
    let result = contract.initial_state(ctor_ctx()).expect("initial_state");
    let view = ledger(&result.current_contract_state);

    assert_eq!(view.field_cell().expect("field_cell"), Fr::from(0u64));
    assert_eq!(view.uint_cell().expect("uint_cell"), 0u64);
    assert_eq!(
        view.field_vec().expect("field_vec"),
        [Fr::from(0u64), Fr::from(0u64)]
    );
    assert_eq!(
        view.hash_cell().expect("hash_cell"),
        midnight_compact_runtime::std_lib::persistent_hash_aligned(&[
            AlignedValue::from(Fr::from(0u64)),
            AlignedValue::from(Fr::from(0u64)),
        ])
    );
    assert_eq!(
        view.point_cell().expect("point_cell"),
        midnight_compact_runtime::hash_to_curve(Fr::from(0u64))
    );
}

/// Every newly-covered value position round-trips: the emitted circuit
/// computes the intended Compact value, so the coercion produced the right
/// Rust type and value at each site.
#[test]
fn every_coerced_position_round_trips() {
    // Return-tail literal.
    assert_eq!(
        pure_circuits::ret_field_literal().expect("ret"),
        Fr::from(0u64)
    );
    // Field const RHS + pure-call argument literal.
    assert_eq!(
        pure_circuits::const_then_call().expect("const+call"),
        Fr::from(0u64)
    );
    // Struct Field-member literal.
    assert_eq!(
        pure_circuits::struct_literal().expect("struct").f,
        Fr::from(0u64)
    );
    // `some<Field>(0)`.
    assert_eq!(
        pure_circuits::some_field_literal().expect("some"),
        midnight_compact_runtime::std_lib::some(Fr::from(0u64))
    );
    // `persistentHash([0, 0])` — literal vector elements.
    assert_eq!(
        pure_circuits::hash_zero_literal().expect("hash"),
        midnight_compact_runtime::std_lib::persistent_hash_aligned(&[
            AlignedValue::from(Fr::from(0u64)),
            AlignedValue::from(Fr::from(0u64)),
        ])
    );
    // Native-argument literal.
    assert_eq!(
        pure_circuits::native_arg_literal().expect("native"),
        midnight_compact_runtime::hash_to_curve(Fr::from(0u64))
    );
    // Scalar Uint→Field (u64 rung).
    assert_eq!(
        pure_circuits::uint_to_field(200).expect("u8->field"),
        Fr::from(200u64)
    );
    // The wide Uint<128> rung is lossless (no reduction modulo the field).
    assert_eq!(
        pure_circuits::u128_rung(1u128 << 100).expect("u128->field"),
        Fr::from(1u128 << 100)
    );
    // Aggregate target: element-wise `Uint<32>` → `Field`.
    assert_eq!(
        pure_circuits::vector_elt_wise(7).expect("vector"),
        [Fr::from(7u64), Fr::from(7u64)]
    );
    // Native-vector element / argument coercion: a var-ref element must be
    // Field-aligned (`Fr::from((x) as u64)`), not passed through as a
    // Byte-aligned `u8`; a nested element or whole argument that is not a
    // tuple literal must bind once and flatten to its leaves rather than wrap
    // a coerced array in `AlignedValue::from` (which has no `From` impl).
    let expected_var_ref_hash = midnight_compact_runtime::std_lib::persistent_hash_aligned(&[
        AlignedValue::from(Fr::from(5u64)),
        AlignedValue::from(Fr::from(0u64)),
    ]);
    assert_eq!(
        pure_circuits::hash_uint_var_ref_elem(5).expect("var-ref elem"),
        expected_var_ref_hash
    );
    assert_eq!(
        pure_circuits::hash_nested_uint_var_ref_elem(5).expect("nested var-ref elem"),
        expected_var_ref_hash
    );
    assert_eq!(
        pure_circuits::hash_var_ref_vector_arg([5u8, 0]).expect("var-ref vector arg"),
        expected_var_ref_hash
    );
    assert_eq!(
        pure_circuits::hash_nested_var_ref_vector_arg([5u8, 0]).expect("nested var-ref vector arg"),
        expected_var_ref_hash
    );
    // Same-typed whole aggregate: no `safe-cast` is emitted, so the emitter
    // must recover the argument's own type. A regression re-wraps the whole
    // `[Fr; N]` in `AlignedValue::from` (no `From` impl) — a `cargo build`
    // failure pinned here by compilation plus the executing assertions.
    assert_eq!(
        pure_circuits::hash_same_type_vector_arg([Fr::from(5u64), Fr::from(0u64)])
            .expect("same-type vector arg"),
        expected_var_ref_hash
    );
    let expected_same_type_nested_hash =
        midnight_compact_runtime::std_lib::persistent_hash_aligned(&[
            AlignedValue::from(Fr::from(5u64)),
            AlignedValue::from(Fr::from(6u64)),
            AlignedValue::from(Fr::from(5u64)),
            AlignedValue::from(Fr::from(6u64)),
        ]);
    assert_eq!(
        pure_circuits::hash_same_type_nested_arg([Fr::from(5u64), Fr::from(6u64)])
            .expect("same-type nested arg"),
        expected_same_type_nested_hash
    );
    assert_eq!(
        pure_circuits::hash_const_vector_arg().expect("const vector arg"),
        expected_var_ref_hash
    );
    assert_eq!(
        pure_circuits::hash_call_vector_arg().expect("call vector arg"),
        expected_var_ref_hash
    );
    assert_eq!(
        pure_circuits::hash_struct_field_vector_arg(VecBox {
            v: [Fr::from(5u64), Fr::from(0u64)],
        })
        .expect("struct-field vector arg"),
        expected_var_ref_hash
    );
    let expected_default_hash = midnight_compact_runtime::std_lib::persistent_hash_aligned(&[
        AlignedValue::from(Fr::from(0u64)),
        AlignedValue::from(Fr::from(0u64)),
    ]);
    assert_eq!(
        pure_circuits::hash_default_vector_arg().expect("default vector arg"),
        expected_default_hash
    );
}

/// Field literals above `u64::MAX` are rendered at the Field width — the
/// `u128` rung, then the little-endian byte constructor above `u128::MAX` —
/// rather than a fixed `u64` literal that overflowed and failed
/// `cargo build` while `compactc` exited 0. Each literal-coercion call site
/// is exercised so no site can regress to the `u64`-only form.
#[test]
fn wide_field_literals_never_overflow_u64() {
    // `u128::MAX` — the top of the `u128` rung.
    assert_eq!(
        pure_circuits::ret_u128_field_literal().expect("u128 rung"),
        Fr::from(u128::MAX)
    );

    // 2^200 — above `u128::MAX`, so it takes the byte constructor. Pinning
    // the little-endian bytes proves the value is exactly the literal, not a
    // wrapped or truncated one.
    let huge = pure_circuits::ret_huge_field_literal().expect("huge literal");
    let mut expected = [0u8; 32];
    expected[25] = 1; // 2^200 = 0x01 << 200
    assert_eq!(huge.as_le_bytes(), expected.to_vec());

    // The remaining literal-coercion sites carry the same value.
    assert_eq!(
        pure_circuits::const_huge_field_literal().expect("const"),
        huge
    );
    assert_eq!(
        pure_circuits::call_arg_huge_field_literal().expect("call arg"),
        huge
    );
    assert_eq!(
        pure_circuits::struct_member_huge_field_literal()
            .expect("struct member")
            .f,
        huge
    );
    assert_eq!(
        pure_circuits::vector_elt_huge_field_literal().expect("vector elt"),
        [huge]
    );
    assert!(pure_circuits::cmp_huge_field_literal(huge).expect("cmp huge"));
    assert!(!pure_circuits::cmp_huge_field_literal(Fr::from(0u64)).expect("cmp zero"));
}

/// Field arithmetic operands are materialised as `Fr` at the typechecker's
/// recorded coercion target. Before the fix, `arith-binop-rust`'s FIELD branch
/// (`mbits = #f`) had no width cast to normalise its operands, so `x + 1`
/// emitted `(x) + (1)` — an untyped Rust integer operand on an `Fr`, which
/// `compactc` accepted (exit 0) but `cargo build` rejected (E0308); a literal
/// above `u64::MAX` additionally overflowed (E0080). A `Uint` operand had the
/// same gap (`(x) + (u: u8)`). Each operator, both literal sides, the
/// "small" and above-`u64::MAX` rungs, a `Uint` operand, and nested
/// arithmetic are covered.
#[test]
fn field_arithmetic_operands_are_materialised() {
    let x = Fr::from(5u64);
    let huge = pure_circuits::ret_huge_field_literal().expect("huge literal");

    // Small literal, both operand sides and `+` / `-`.
    assert_eq!(
        pure_circuits::add_field_literal(x).expect("add"),
        Fr::from(6u64)
    );
    assert_eq!(
        pure_circuits::lit_lhs_field_literal(x).expect("add lhs"),
        Fr::from(6u64)
    );
    assert_eq!(
        pure_circuits::sub_field_literal(x).expect("sub"),
        Fr::from(4u64)
    );

    // A literal above `u64::MAX` takes the byte rung inside the operand.
    assert_eq!(
        pure_circuits::add_huge_field_literal(x).expect("add huge"),
        x + huge
    );
    assert_eq!(
        pure_circuits::mul_huge_field_literal(x).expect("mul huge"),
        x * huge
    );

    // A `Uint` operand is coerced losslessly to Field.
    assert_eq!(
        pure_circuits::add_uint_field_operand(x, 7u8).expect("add uint"),
        x + Fr::from(7u64)
    );

    // Nested arithmetic materialises every operand at every level.
    assert_eq!(
        pure_circuits::nested_field_arith(x).expect("nested"),
        (x + Fr::from(1u64)) * (x + Fr::from(2u64))
    );
}

/// Field literals above `max-unsigned` (2^248 - 1) are materialised as `Fr`
/// even where the position supplies no expected type. The typechecker admits
/// such a literal only as `N as Field` and lowers it without a `safe-cast`, so
/// a call argument (native or pure circuit) used to print bare digits no Rust
/// integer holds: `compactc` exited 0 and `cargo build` failed (#90). The
/// 2^200 literals above are below `max-unsigned` and never took that path.
#[test]
fn field_only_literals_materialise_without_an_expected_type() {
    // The Jubjub prime-subgroup order `r`, as little-endian u64 limbs.
    const JUBJUB_R: [u64; 4] = [
        0xd097_0e5e_d6f7_2cb7,
        0xa668_2093_ccc8_1082,
        0x0667_3b01_0134_3b00,
        0x0e7d_b4ea_6533_afa9,
    ];
    // `(r + 1) / 8` in plain integer arithmetic: `r` is not the Field
    // modulus, so this cannot be computed with `Fr` operations.
    let mut limbs = JUBJUB_R;
    let mut carry = 1u64;
    for limb in limbs.iter_mut() {
        let (sum, overflow) = limb.overflowing_add(carry);
        *limb = sum;
        carry = overflow as u64;
    }
    assert_eq!(carry, 0);
    assert_eq!(limbs[0] & 0b111, 0, "8 divides r + 1");
    let mut shifted_in = 0u64;
    for limb in limbs.iter_mut().rev() {
        let low_bits = *limb << 61;
        *limb = (*limb >> 3) | shifted_in;
        shifted_in = low_bits;
    }
    let expected: Vec<u8> = limbs.iter().flat_map(|l| l.to_le_bytes()).collect();

    // Every position carries exactly the literal.
    let c = pure_circuits::ret_field_only_literal().expect("return");
    assert_eq!(c.as_le_bytes(), expected);
    assert_eq!(
        pure_circuits::call_arg_field_only_literal().expect("call arg"),
        c
    );
    assert_eq!(pure_circuits::const_field_only_literal().expect("const"), c);
    assert_eq!(
        pure_circuits::add_field_only_literal(Fr::from(0u64)).expect("add"),
        c
    );
    assert!(pure_circuits::cmp_field_only_literal(c).expect("cmp c"));
    assert!(!pure_circuits::cmp_field_only_literal(Fr::from(0u64)).expect("cmp zero"));
    let g = midnight_compact_runtime::ec_mul_generator(Fr::from(1u64));
    assert_eq!(
        pure_circuits::native_arg_field_only_literal(g).expect("native arg"),
        midnight_compact_runtime::ec_mul(g, c)
    );

    // The boundary: 2^248 = 0x01 << 248, the smallest Field-only literal.
    let mut boundary = [0u8; 32];
    boundary[31] = 1;
    assert_eq!(
        pure_circuits::call_arg_max_unsigned_plus_one()
            .expect("boundary")
            .as_le_bytes(),
        boundary.to_vec()
    );

    // `8 * c = 1 mod r`, so scaling a subgroup point by `c` then 8 is the
    // identity: the credential-compact subgroup check accepts the generator.
    assert_eq!(pure_circuits::subgroup_check().expect("subgroup check"), g);
}
