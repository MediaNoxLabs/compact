# Bounded-Uint Write Alignment — Phase F.2 Investigation

**Date:** 2026-06-29 (worktree HEAD: `c6c73c0`, branch `codegen-rust`)
**Status:** Escalated. Original gap (`Uint<8>` write from `Set.size()`) does NOT reproduce. A related but distinct gap exists for `Uint<L..U>` ranges with non-power-of-2 upper bounds, and the fix is larger than 50 LOC of Scheme.

## TL;DR

The A20 worker's note about "bounded-Uint write alignment broken" no longer reproduces for power-of-2-width `Uint<N>` writes from `u64` sources (e.g. `s.size()`). The current codegen correctly emits a `u8`-aligned cell.

A distinct gap remains: **bounded-range writes with non-power-of-2 upper bounds** (e.g. `Uint<0..100>`, `Uint<0..70000>`) where the rhs is anything more complex than a literal — the walker pre-rejects the body via `tunsigned-rust-suffix-for-bound` and emission never runs.

## Repro

Worktree compactc (`result/bin/compactc`) at HEAD `c6c73c0`. Outputs under `/tmp/bounded-uint-investigation/`.

### Case A — `Uint<8>` from `Set.size()` — WORKS ✅

`a20natural.compact`:
```compact
pragma language_version 0.23;
import CompactStandardLibrary;

export ledger s: Set<Field>;
export ledger n: Uint<8>;

export circuit populate(): [] {
  n.write(disclose(s.size() as Uint<8>));
}
```

Compiles cleanly under both `--rust` and the default TS. Rust emission:

```rust
let tmp = ({
    let _gather_ops = OpProgramGather::<DefaultDB>::new()
        .dup(0).idx_at_index(0u8, false).size().popeq(true).build();
    /* ... query_for_read ... */
    compact_runtime::std_lib::decode_u64(_av)?
}) as u8;
let ops = OpProgramVerify::<DefaultDB>::new()
    .push(false, new_cell(1u8))
    .push(true, new_cell(tmp.clone()))
    .ins(false, 1)
    .build();
```

`tmp: u8`, so `new_cell(tmp.clone())` produces a 1-byte-aligned `AlignedValue` via the `Aligned for u8 { length: 1 }` impl in `midnight-base-crypto/src/fab/alignments.rs`. TS-side `_descriptor_0 = CompactTypeUnsignedInteger(255n, 1)` produces the same 1-byte alignment. **Byte-parity holds.**

The A20 worker's "had to scope down to Boolean writes" appears to have been addressed (or always worked for `Uint<8>`) — at least with the current HEAD the simple `Uint<8>` size-write shape lowers correctly.

### Case B — `Uint<0..100>` from `Set.size()` — FAILS ❌

`asu8.compact`:
```compact
export ledger n: Uint<0..100>;
export circuit from_size(): [] {
  n.write(disclose(s.size() as Uint<0..100>));
}
```

```
Exception: asu8.compact line 7 char 1:
  compactc --rust: unsupported Compact construct (circuit-body-emission): no walker shape matched
  circuit body for from_size
```

### Case C — `Uint<0..255>` from `Set.size()` — FAILS ❌

Same error. So failure isn't specific to byte-width — it's specific to the `Uint<L..U>` *form* with a non-standard upper bound (255 is power-of-2-1, but it's spelled as a range rather than `Uint<8>`).

### Case D — `Uint<0..255>` from a literal — WORKS ✅

`litwrite.compact` with `n.write(disclose(5 as Uint<0..255>))` compiles. The IR likely folds the literal cast at typecheck time, so no `downcast-unsigned` survives to the walker.

## Classification

**Case D from the task spec — something else entirely.**

The alignment is NOT wrong when emission succeeds. The gap is upstream: the walker's `tunsigned-rust-suffix-for-bound` (in `compiler/rust-passes-walker.ss:464-471`) only accepts nat ∈ {255, 65535, 4294967295, 18446744073709551615, 340282366920938463463374607431768211455}. Every other nat falls through to `#f`, and the surrounding `(downcast-unsigned ,src ,nat? ,nat ,expr)` clauses in `expr-supported?` / `lambda-body-supported?` (lines 823-834, 947-949) consequently reject the expression. The body walker then sees no matching shape for the circuit and bails with the catch-all error.

For non-literal rhs, the walker can never even attempt emission for `Uint<L..U>` with `U ∉ {2^k - 1 : k ∈ {8,16,32,64,128}}`. The original wording about "wrong alignment" implies the codegen runs and emits the wrong bytes — but in fact it doesn't run at all.

## Why the fix is bigger than 50 LOC of Scheme

A minimal patch would need:

1. **Generalise `tunsigned-rust-suffix-for-bound`** to return the smallest standard Rust width whose `u128 → uN` cast can hold `nat`, for any nat ≤ u128::MAX. ~5 LOC. *Small.*

2. **Decide cell-builder selection in the write path.** TS encodes the value through `_descriptor_N.toValue(tmp)` where the descriptor has byte-width `ceil(log2(U+1)/8)` — which does **not** always equal the Rust integer width (e.g. `Uint<0..70000>` needs 17 bits = 3 bytes but is held in a `u32`). The Rust codegen currently has two cell builders:
   - `new_cell::<T>` — uses `T: Into<AlignedValue>`, alignment from `Aligned for T` (u8→1, u16→2, u32→4, u64→8, u128→16). Always power-of-2.
   - `new_cell_bounded_uint(value: u128, byte_len: usize)` — explicit byte_len. Already used by `initial_state` for bounded-range zeros (see `bounded-uint-fixture/lib.rs:63-64`).

   For an `as Uint<L..U>` write whose byte_len doesn't match the Rust integer width, the emitter must switch to `new_cell_bounded_uint(tmp as u128, byte_len)`. This crosses both the `expr-supported?` predicate and the `push(true, ...)` emission in `emit-body-mutations` / `ctor-expr-rust`. Touches >1 file. Probably ~80-150 LOC. *Medium.*

3. **Reconcile the gather/decode path.** When reading a bounded-range cell back, `decoder-for-type` (rust-passes-emit.ss:2300-2308) already maps any nat to the next u8/u16/u32/u64/u128 decoder, so reads are fine. But the value bytes on-chain are byte_len-wide (not Rust-integer-wide), and the existing `decode_uN` helpers assume the AlignedValue's atom has the full Rust width's worth of bytes. Need to verify this round-trips for the `byte_len < Rust width` case — e.g. `Uint<0..70000>` reads back into `u32` from a 3-byte cell. *Investigate; possibly a runtime helper gap.*

Items 2 and 3 are the budget overrun. The right structural fix likely belongs in a follow-up iteration (call it A22 or B-bug-something) that pairs the walker change with a `new_cell_bounded_uint`-aware emit clause and a byte_len-respecting read helper.

## Recommended next steps

- Leave `tunsigned-rust-suffix-for-bound` as-is (the conservative gate is correctly catching shapes we'd otherwise miscompile).
- File the `Uint<L..U>` bounded-range circuit-body gap as a tracked walker gap (post-A21, sibling to A20).
- For the original A20 fixture intent: if the test wants size-write coverage and the existing `Uint<8>` form works, regenerate the fixture using `Uint<8>` rather than scoping down to Boolean. (Out of scope for this investigation — no fixture changes attempted.)

## Reproducer artifacts

`/tmp/bounded-uint-investigation/`:
- `baseline.compact` / `baseline/` — `Uint<8>` literal write, works.
- `fromsize.compact` / `fromsize/` — `Uint<8>` from `s.size()`, works.
- `a20natural.compact` / `a20natural/` — the "natural A20" shape: `Set<Field> + Uint<8> + size() write`, works.
- `litwrite.compact` / `litwrite/` — `Uint<0..255>` literal write, works.
- `range.compact` / `asu8.compact` / `largerange.compact` — bounded-range from `s.size()`, fail with walker error.
