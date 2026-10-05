# Shielded merge oracle

The source calls unchanged `mergeCoin` and `receiveShielded` followed by
`mergeCoinImmediate`. The independent capture uses the corrected pinned TS
runtime with the 16-byte coin-value descriptor.

The 20 TS/native cases retain normal and swapped inputs, zero values, exact
u128 maximum, values above u64, overflowing sums, wrong colors (including
color-before-overflow precedence), and duplicate coin identities. Rejections
retain only successful TS query/intent prefixes; no returned post-error context
or private transcript is invented. Gas retains summed query, wrapper-last-query
and whole-program replay costs separately.

The upstream construction test uses real historical inputs, a real wallet input,
an actual transient and a merged contract output. It checks source-order versus
canonical allocation, frontier and nullifiers. Duplicate raw execution succeeds,
while the upstream offer refuses its repeated nullifier. This is allocation and
identity evidence, **not** strict proof/ledger application evidence.

At the ADR207 preparation checkpoint, both generated exports remain explicitly
recording-unavailable. The unregistered proof-smoke `shielded_merge.rs` module is
preparation for two funded default-strict proof paths and is not yet compiled or
executed. Shared planner work follows ADR206's signed handoff; ABI48 output here
will be refreshed to that integration baseline. No runtime changes are prepared.
