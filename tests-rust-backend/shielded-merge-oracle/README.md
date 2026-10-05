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

## Recorded delivery

Both proof-required exports now provide typed recorded and observed-call APIs.
The same shared planner materializes arguments once, checks the two-limb
129-bit addition bounds, and preserves the original color assertion before
narrowing. The input policy distinguishes two historical qualified inputs from
one historical input plus the same received coin qualified at singleton index
zero. A separate audit checks every helper, argument, binding and branch.

The 20 cases compare native and recorded result, state, effects, gas, private
outputs and complete public operations against the independent TS evidence.
Successful programs replay through the upstream VM with separately measured
whole-program gas. Failure cases assert native/recorded diagnostics and retain
TS successful prefixes without inventing post-error contexts.

`compact-rust-proof-smoke --shielded-merge <output>` proves both exports (4,480
bytes each), uses real historical/wallet/transient carriers and an actual
merged output, funds fees with separate Dust, then checks default-strict
well-formedness, ledger application, canonical output index/owner and nullifier
replay refusal. Changed circuit binding also rejects. The initial historical
coins are explicitly seeded offline; this is not a proved deposit history.
Both cases start at frontier 2. The immediate case retains the distinction
between raw TS source-order indices and authoritative upstream output/transient
allocation.

The preparation checkpoint intentionally had native-only ABI48 output and an
unregistered proof draft. Delivery is ABI49/schema20 inherited from ADR206;
merge adds no runtime API, allocation policy, schema or ABI revision.
Original application composition (microDAO set_topic/buy_in and Coracle start),
including fallible transient funding, remains separately scoped.
