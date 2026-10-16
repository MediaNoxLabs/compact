# Qualified coin recording from fresh source

The pinned Compact frontend generates the retained schema20 IR from
`examples/rust_backend/effect_plan_qualified_coin_source.compact`. This fixture
renders that IR with the current Rust backend on every build; generated Rust is
not checked in or edited.

`write_once` uses the existing parameterless Unit recording domain and witnesses
for both operands. `write_read` reconstructs its returned struct from a single
local ledger read to use the existing composite expression-return domain. A
direct `return pot` becomes a different frontend return variant outside that
selector domain. The
compiler tests separately assert these actual algebra owners. The sixteen-field
source puts `pot` at physical path `[1, 14]`, checked during the fixture build.

Tests compare native execution, generated recording, a manual official
`RecordingFrame`/slot reference, and public transcript replay. They cover both
recipient arms, zero and maximal coin values/allocation indices, replacement,
witness evaluation order and private outputs, callback failure prefixes, missing
allocation, and a commitment allocated for the wrong recipient. A manually seeded
`com_indices` entry is a focused runtime fixture, not funding or proof validation.

The full-payload amount/index matrix has 12 accepted and 6 rejected cases. The
existing ledger8 runtime uses `Concat { n: 91 }`: acceptance depends on the sum
of serialized official coin and index sizes, not simply their Rust numeric
widths. Separate probes cover exact 91-byte acceptance, 92-byte rejection,
and simultaneous numeric maxima with shorter normalized nonce/color payloads.
Rejections must agree across native, recorded and manual execution and preserve
retained snapshots. This inherited runtime bound is not loosened here.

Sequential execution costs and batched transcript-replay costs are checked
separately because official ledger queries charge per-invocation state-update
overhead.

No selector retirement or signature expansion is implied by this fixture.
