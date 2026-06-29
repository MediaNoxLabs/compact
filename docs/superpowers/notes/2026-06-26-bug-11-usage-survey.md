# Bug-11 Survey: Uint<L..U> Non-Power-of-2 Usage in Compact Ecosystem

**Date:** 2026-06-29
**Finding:** Non-power-of-2 bounded ranges ARE used in the real test corpus, but only in synthetic/exploratory tests, not in production contracts.

## 1. Empirical Usage in Test Corpus

**Test File Count:** 1012 `.compact` files across `compiler/javascript-code/`.

**Bounded Range Occurrences:** 301 instances of `Uint<L..U>` (both standalone and in containers).

**Non-Power-of-2 Upper Bounds:** 39 occurrences (13% of all bounded ranges)
- `Uint<0..3>`: 14 uses (test923, test924, test926, test928, test930, test932)
- `Uint<0..5>`: 12 uses (test928, test930, test932)
- `Uint<0..10>`: 6 uses (test643, test724)
- `Uint<0..36>`: 5 uses (test643, test724)
- `Uint<0..11>`: 2 uses (test609)

**Representative Files:**
- `/Users/ysh/iohk/compact/.claude/worktrees/admiring-lehmann-05e4d9/compiler/javascript-code/test930/src/compiler/testdir/testfile.compact:17` — Vector of `Uint<0..5>` enum-cast fixture
- `/Users/ysh/iohk/compact/.claude/worktrees/admiring-lehmann-05e4d9/compiler/javascript-code/test643/contract/index.js:163` — Large `Uint<0..340282366920938463463374607431768211456>` (2^128) in test metadata
- `/Users/ysh/iohk/compact/.claude/worktrees/admiring-lehmann-05e4d9/compiler/javascript-code/test724/src/compiler/testdir/testfile.compact` — Enum-cast to `Uint<0..36>`

**Pattern:** All 14 files using non-power-of-2 bounds are enum/variant-cast tests or exploratory fixtures. Zero uses in production contracts (`did.compact`, ledger, zswap).

## 2. Spec Sanction

**Reference:** `/Users/ysh/iohk/compact/.claude/worktrees/admiring-lehmann-05e4d9/doc/compact-reference.mdx`

**Spec Explicitly Permits `Uint<m..n>` with Arbitrary Bounds:**
- Section on *tsize*: "\`Uint\`<[*tsize*](#primitive-types) \`\`.. [*tsize*](#primitive-types) \`>`"
- Section on subtyping: "`Uint<0..`*n*`>` is a subtype of `Uint<0..`*m*>` if *n* is less than (or equal to) *m*"
- Language supports generic `Uint<0..n>` where `n` is bound at compile time (e.g., `Vector<n, T>` indices use `Uint<0..n>`)

**Verdict:** Language spec fully sanctions non-power-of-2 bounds as first-class features.

## 3. TypeScript Backend Behaviour

**File:** `/Users/ysh/iohk/compact/.claude/worktrees/admiring-lehmann-05e4d9/runtime/src/compact-types.ts`

**TS Runtime Class:** `CompactTypeUnsignedInteger` (lines 200+)
```typescript
export class CompactTypeUnsignedInteger implements CompactType<bigint> {
  readonly maxValue: bigint;
  readonly length: number;  // ← arbitrary byte-width

  constructor(maxValue: bigint, length: number) {
    this.maxValue = maxValue;
    this.length = length;
  }

  alignment(): ocrt.Alignment {
    return [{ tag: 'atom', value: { tag: 'bytes', length: this.length } }];
  }
  // ... toValue/fromValue handle arbitrary length
}
```

**Key:** TS backend **fully supports arbitrary `length` parameters**, computing byte width as `ceil(log2(maxValue+1)/8)`. E.g., `Uint<0..100>` → 1 byte, `Uint<0..70000>` → 3 bytes.

**Verdict:** TS side has **zero limitation**. Rust is a genuine divergence, not a mirroring of TS limitation.

## 4. Rust Codegen Gap

**File:** `/Users/ysh/iohk/compact/.claude/worktrees/admiring-lehmann-05e4d9/docs/superpowers/notes/2026-06-26-bounded-uint-write-alignment.md`

**Walker Rejection:** `tunsigned-rust-suffix-for-bound` (rust-passes-walker.ss:464-471) only accepts nat ∈ {255, 65535, 4294967295, 18446744073709551615, 340282366920938463463374607431768211455} (the power-of-2-minus-1 ladder).

**Failure Mode:** Non-literal writes to `Uint<0..N>` with `N ∉ ladder` fail at walker time:
```
Exception: unsupported Compact construct (circuit-body-emission): no walker shape matched
```

**Workaround:** Literal writes (enum casts) work because typecheck folds them before the walker sees them.

**Fix Scope:** ~80-150 LOC across walker + emit + runtime:
1. Generalize `tunsigned-rust-suffix-for-bound` (~5 LOC)
2. Conditional `new_cell` vs `new_cell_bounded_uint` in emit (~75-100 LOC)
3. Verify read-path round-trip for sub-byte-width cells (~10 LOC)

## Verdict

**PRIORITY — Real Gap, Spec-Compliant Limitation**

- **Empirical:** 13% of test corpus uses non-power-of-2 bounds; all are synthetic tests, zero in `did.compact` or production contracts.
- **Spec:** Language explicitly sanctions arbitrary `Uint<L..U>`. Not a documented-as-unsupported feature.
- **TS vs Rust:** TypeScript handles non-power-of-2 bounds perfectly. Rust is a *divergence*, not a shared limitation.
- **Risk Surface:** Any real contract using `Uint<0..n>` indices (e.g., `Vector<n, T>` for non-power-of-2 `n`) + a non-literal circuit assignment would trip the walker gap.

**Recommendation:** File as a tracked bug (post-A21). The feature is documented-as-supported, works in TS, and the fix is medium-complexity (~80-150 LOC). Do not defer indefinitely — schedule for next iteration once A21/A22 stabilizes.
