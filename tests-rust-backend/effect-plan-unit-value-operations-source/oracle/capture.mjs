// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
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
const root = new URL(".", import.meta.url).pathname;
const generated = process.argv[2];
const runtime = process.argv[3];
assert.ok(
  generated && runtime,
  "usage: capture.mjs GENERATED_CONTRACT_DIR OFFICIAL_TS_RUNTIME_DIR",
);
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
const identity = { x: 0n, y: 1n },
  nonidentity = r.hashToCurve(r.CompactTypeField, 1n),
  rows = [],
  queries = [],
  oldQuery = r.QueryContext.prototype.query;
r.QueryContext.prototype.query = function (...args) {
  const result = oldQuery.call(this, ...args);
  queries.push(result.gasCost);
  return result;
};
const sum = (rows) =>
  Object.fromEntries(
    ["readTime", "computeTime", "bytesWritten", "bytesDeleted"].map((k) => [
      k,
      rows.reduce((n, x) => n + x[k], 0n),
    ]),
  );

const modulus = r.MAX_FIELD + 1n;
const zeroBytes = new Uint8Array(32);
function run({
  id,
  circuit = "evaluate",
  x = nonidentity.x + 7n,
  y = nonidentity.y,
  offset = 7n,
  fallback = identity,
  selected = true,
  permit = true,
  fail = null,
  expected = nonidentity,
  blank = zeroBytes,
  boundary = "parity",
}) {
  const events = [];
  function witness(name, value) {
    return (ctx) => {
      events.push([name, ctx.privateState, ctx.ledger.calls]);
      if (fail === name) throw Error(`${name} provider refused`);
      return [ctx.privateState + 1, value];
    };
  }
  const contract = new Contract({
    coordinate_x: witness("coordinate_x", x),
    coordinate_y: witness("coordinate_y", y),
    permit: witness("permit", permit),
  });
  const key = { bytes: new Uint8Array(32) };
  const initial = contract.initialState({
    initialPrivateState: 5,
    initialZswapLocalState: r.emptyZswapLocalState(key),
  });
  const ctx = r.createCircuitContext(
    r.dummyContractAddress(),
    key,
    initial.currentContractState.data,
    5,
  );
  const args =
    circuit === "close"
      ? [offset, expected, blank]
      : [offset, fallback, selected];
  const difference = (x - offset + modulus) % modulus;
  const point =
    circuit === "close" || selected ? { x: difference, y } : fallback;
  const expectedEvents = [];
  let reason = null;
  if (circuit === "close" || selected) {
    for (const name of ["coordinate_x", "coordinate_y"]) {
      expectedEvents.push([name, 5 + expectedEvents.length, 1n]);
      if (fail === name) {
        reason = `${name} provider refused`;
        break;
      }
    }
  }
  if (!reason && circuit === "close") {
    if (point.x !== expected.x) reason = "wrong x";
    else if (point.y !== expected.y) reason = "wrong y";
    else if (blank.some((v) => v !== 0)) reason = "wrong default";
  }
  if (!reason) {
    expectedEvents.push(["permit", 5 + expectedEvents.length, 1n]);
    if (fail === "permit") reason = "permit provider refused";
    else if (!permit) reason = "denied";
  }
  const start = queries.length;
  let call, error;
  try {
    call = contract.circuits[circuit](ctx, ...args);
  } catch (e) {
    error = String(e);
  }
  assert.deepEqual(events, expectedEvents, id);
  const row = {
    id,
    circuit,
    x: clean(x),
    y: clean(y),
    offset: clean(offset),
    fallback: clean(fallback),
    selected,
    permit,
    fail,
    expected: clean(expected),
    blank: clean(blank),
    boundary,
    difference: clean(difference),
    point: clean(point),
    args: clean(args),
    events: clean(events),
    queryCostSum: clean(sum(queries.slice(start))),
  };
  if (reason) {
    assert.ok(error?.includes(reason), `${id}: ${error}`);
    rows.push({ ...row, status: "refused", reason, error });
    return;
  }
  assert.equal(error, undefined, id);
  assert.deepEqual(
    call.result,
    circuit === "close" ? [] : { ...point, blank: zeroBytes },
    id,
  );
  assert.equal(call.context.currentPrivateState, 5 + events.length, id);
  assert.equal(ledger(call.context.currentQueryContext.state).calls, 1n, id);
  const privateExpected = [];
  if (circuit === "close" || selected)
    for (const value of [x, y])
      privateExpected.push({
        value: r.CompactTypeField.toValue(value),
        alignment: r.CompactTypeField.alignment(),
      });
  privateExpected.push({
    value: r.CompactTypeBoolean.toValue(true),
    alignment: r.CompactTypeBoolean.alignment(),
  });
  assert.deepEqual(
    call.proofData.privateTranscriptOutputs,
    privateExpected,
    id,
  );
  const preimage = r.proofDataIntoSerializedPreimage(
    call.proofData.input,
    call.proofData.output,
    call.proofData.publicTranscript,
    call.proofData.privateTranscriptOutputs,
    circuit,
  );
  rows.push({
    ...row,
    status: "success",
    result: clean(call.result),
    privateState: call.context.currentPrivateState,
    proofData: clean(call.proofData),
    gasCost: clean(call.gasCost),
    serializedPreimage: { status: "success", bytes: preimage.length },
  });
}
run({ id: "evaluate-identity", x: 0n, y: 1n, offset: 0n });
run({ id: "evaluate-nonidentity" });
run({ id: "evaluate-modular-wrap", x: 0n, offset: modulus - nonidentity.x });
run({
  id: "evaluate-lazy-suppressed",
  selected: false,
  fallback: nonidentity,
  fail: "coordinate_x",
});
run({
  id: "evaluate-invalid-coordinates",
  x: 1n,
  y: 1n,
  offset: 0n,
  boundary:
    "TS accepts raw coordinates; existing Rust rejects InvalidJubjubPoint before permit",
});
run({ id: "evaluate-x-refusal", fail: "coordinate_x" });
run({ id: "evaluate-y-refusal", fail: "coordinate_y" });
run({ id: "evaluate-permit-refusal", fail: "permit" });
run({ id: "evaluate-denied", permit: false });
run({ id: "close-nonidentity", circuit: "close" });
run({ id: "close-wrong-x", circuit: "close", expected: identity });
run({
  id: "close-wrong-default",
  circuit: "close",
  blank: Uint8Array.from({ length: 32 }, (_, i) => (i === 0 ? 1 : 0)),
});
assert.equal(new Set(rows.map((v) => v.id)).size, 12);
const files = [
  new URL(
    "../../../examples/rust_backend/effect_plan_unit_value_operations.compact",
    import.meta.url,
  ),
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
const counts = {
  cases: rows.length,
  executionSuccess: rows.filter((v) => v.status === "success").length,
  executionRefusal: rows.filter((v) => v.status === "refused").length,
  preimageSuccess: rows.filter(
    (v) => v.serializedPreimage?.status === "success",
  ).length,
};
writeFileSync(
  join(root, "typescript.json"),
  JSON.stringify(
    {
      status: "passed",
      scope:
        "Genuine frozen default-v2 source execution; bigint modular subtraction and explicit zero Bytes32 reference, official TS constructor accepts invalid coordinates unlike existing checked Rust constructor. No Rust execution or proof claim.",
      files: Object.fromEntries(
        files.map((p) => [
          p,
          createHash("sha256").update(readFileSync(p)).digest("hex"),
        ]),
      ),
      ...counts,
      rows,
    },
    null,
    2,
  ) + "\n",
);
console.log(JSON.stringify({ status: "passed", ...counts }));
