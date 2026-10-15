// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Usage: node capture.mjs SOURCE GENERATED_CONTRACT_DIR RUNTIME_PACKAGE OUTPUT
import assert from "node:assert/strict";
import {
  readFileSync,
  writeFileSync,
  mkdirSync,
  existsSync,
  symlinkSync,
  readdirSync,
  lstatSync,
} from "node:fs";
import { createHash } from "node:crypto";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
const [source, generated, runtime, output] = process.argv.slice(2);
const links = join(generated, "node_modules/@midnight-ntwrk");
mkdirSync(links, { recursive: true });
if (!existsSync(join(links, "compact-runtime")))
  symlinkSync(runtime, join(links, "compact-runtime"));
const r = await import(pathToFileURL(join(runtime, "dist/index.js"))),
  { Contract, ledger } = await import(
    pathToFileURL(join(generated, "index.js"))
  );
const clean = (v) =>
  JSON.parse(
    JSON.stringify(v, (_, x) =>
      typeof x === "bigint"
        ? x.toString()
        : x instanceof Uint8Array
          ? [...x]
          : x,
    ),
  );
const bytes = (v) => Uint8Array.from(v),
  rows = [];
const queries = [],
  oldQuery = r.QueryContext.prototype.query;
r.QueryContext.prototype.query = function (...a) {
  const out = oldQuery.call(this, ...a);
  queries.push(out.gasCost);
  return out;
};
const sum = (q) =>
  Object.fromEntries(
    ["readTime", "computeTime", "bytesWritten", "bytesDeleted"].map((k) => [
      k,
      q.reduce((n, x) => n + x[k], 0n),
    ]),
  );
const B = r.CompactTypeOpaqueUint8Array,
  S = r.CompactTypeOpaqueString,
  F = r.CompactTypeField;
const envelopeType = {
  alignment: () => [
    ...S.alignment(),
    ...B.alignment(),
    ...F.alignment(),
    ...B.alignment(),
    ...B.alignment(),
  ],
  toValue: (v) => [
    ...S.toValue(v.payload.text),
    ...B.toValue(v.payload.bytes),
    ...F.toValue(v.nonce),
    ...B.toValue(v.samples[0]),
    ...B.toValue(v.samples[1]),
  ],
};
const hashes = (type, value) => {
  const p = r.hashToCurve(type, value);
  return {
    x: p.x,
    y: p.y,
    transient: r.transientHash(type, value),
    committed: r.transientCommit(type, value, 7n),
  };
};
function run({
  id,
  circuit = "witnessed",
  payload = [],
  fallback = [],
  selected = true,
  permit = true,
  fail = null,
  envelope = null,
}) {
  const events = [];
  const c = new Contract({
      bytes: (ctx) => {
        events.push(["bytes", ctx.privateState, ctx.ledger.calls]);
        if (fail === "bytes") throw Error("bytes provider refused");
        return [ctx.privateState + 1, bytes(payload)];
      },
      permit: (ctx) => {
        events.push(["permit", ctx.privateState, ctx.ledger.calls]);
        if (fail === "permit") throw Error("permit provider refused");
        return [ctx.privateState + 1, permit];
      },
    }),
    key = { bytes: new Uint8Array(32) };
  const initial = c.initialState({
      initialPrivateState: 5,
      initialZswapLocalState: r.emptyZswapLocalState(key),
    }),
    ctx = r.createCircuitContext(
      r.dummyContractAddress(),
      key,
      initial.currentContractState.data,
      5,
    ),
    args = circuit === "nested" ? [envelope] : [bytes(fallback), selected];
  const start = queries.length;
  let call, error;
  try {
    call = c.circuits[circuit](ctx, ...args);
  } catch (e) {
    error = String(e);
  }
  const queryCostSum = sum(queries.slice(start)),
    expectedEvents = [];
  let reason = null;
  if (circuit === "witnessed") {
    if (selected) {
      expectedEvents.push(["bytes", 5, 1n]);
      if (fail === "bytes") reason = "bytes provider refused";
    }
    if (!reason) {
      expectedEvents.push(["permit", 5 + expectedEvents.length, 1n]);
      if (fail === "permit") reason = "permit provider refused";
      else if (!permit) reason = "denied";
    }
  }
  assert.deepEqual(events, expectedEvents, id);
  const row = {
    id,
    circuit,
    payload,
    fallback,
    selected,
    permit,
    fail,
    envelope: clean(envelope),
    args: clean(args),
    events: clean(events),
    queryCostSum: clean(queryCostSum),
  };
  if (reason) {
    assert.ok(error?.includes(reason), `${id}: ${error}`);
    rows.push({ ...row, status: "refused", reason, error });
    return;
  }
  assert.equal(error, undefined, id);
  assert.deepEqual(
    call.result,
    hashes(
      circuit === "nested" ? envelopeType : B,
      circuit === "nested" ? envelope : bytes(selected ? payload : fallback),
    ),
    id,
  );
  assert.equal(ledger(call.context.currentQueryContext.state).calls, 1n, id);
  assert.equal(call.context.currentPrivateState, 5 + events.length, id);
  let serializedPreimage;
  try {
    const encoded = r.proofDataIntoSerializedPreimage(
      call.proofData.input,
      call.proofData.output,
      call.proofData.publicTranscript,
      call.proofData.privateTranscriptOutputs,
      circuit,
    );
    serializedPreimage = { status: "success", bytes: encoded.length };
  } catch (e) {
    serializedPreimage = { status: "refused", error: String(e) };
  }
  rows.push({
    ...row,
    status: "success",
    result: clean(call.result),
    privateState: call.context.currentPrivateState,
    proofData: clean(call.proofData),
    gasCost: clean(call.gasCost),
    serializedPreimage,
  });
}
for (const [id, payload] of [
  ["empty", []],
  ["zero", [0]],
  ["interior", [1, 0, 2]],
  ["trailing", [1, 0]],
])
  run({ id: `witness-${id}`, payload });
run({
  id: "lazy-provider-suppressed",
  selected: false,
  fail: "bytes",
  payload: [1, 0],
  fallback: [1, 0, 2],
});
run({ id: "lazy-trailing-fallback", selected: false, fallback: [1, 0] });
for (const selected of [false, true]) {
  run({ id: `denied-${selected}`, selected, permit: false });
  run({ id: `permit-refusal-${selected}`, selected, fail: "permit" });
}
run({ id: "bytes-refusal", fail: "bytes" });
const envelope = (text, b, nonce, s0, s1) => ({
  payload: { text, bytes: bytes(b) },
  nonce,
  samples: [bytes(s0), bytes(s1)],
});
for (const [id, value] of [
  ["empty", envelope("", [], 0n, [], [])],
  ["canonical", envelope("λ\0key", [1, 0, 2], 1n, [2], [3, 0, 4])],
  ["text-trailing", envelope("a\0", [1], 2n, [2], [3])],
  ["bytes-trailing", envelope("a", [1, 0], 2n, [2], [3])],
  ["vector-trailing", envelope("a", [1], 2n, [2], [3, 0])],
])
  run({ id: `nested-${id}`, circuit: "nested", envelope: value });
const files = [
  source,
  new URL(import.meta.url),
  join(generated, "compact-rust-ir.json"),
  join(generated, "index.js"),
  join(runtime, "package.json"),
  join(runtime, "package-lock.json"),
];
function tree(dir) {
  for (const n of readdirSync(dir).sort()) {
    const p = join(dir, n),
      s = lstatSync(p);
    if (s.isDirectory()) tree(p);
    else if (s.isFile()) files.push(p);
  }
}
tree(join(runtime, "dist"));
tree(join(runtime, "node_modules/@midnight-ntwrk/onchain-runtime-v3"));
const report = {
  status: "passed",
  scope:
    "Fresh genuine default-v2 source TS execution plus exact official primitive answers. Serialization attempts are separate from execution and do not claim proof generation. No Rust qualification performed here.",
  files: Object.fromEntries(
    files.map((p) => [
      p,
      createHash("sha256").update(readFileSync(p)).digest("hex"),
    ]),
  ),
  caseCount: rows.length,
  rows,
};
writeFileSync(output, JSON.stringify(report, null, 2) + "\n");
console.log(
  JSON.stringify({
    status: "passed",
    cases: rows.length,
    executionSuccess: rows.filter((x) => x.status === "success").length,
    executionRefusal: rows.filter((x) => x.status === "refused").length,
    preimageSuccess: rows.filter(
      (x) => x.serializedPreimage?.status === "success",
    ).length,
    preimageRefusal: rows.filter(
      (x) => x.serializedPreimage?.status === "refused",
    ).length,
  }),
);
