// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Usage: node capture.mjs SOURCE GENERATED_CONTRACT_DIR RUNTIME_PACKAGE OUTPUT
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import {
  readFileSync,
  writeFileSync,
  mkdirSync,
  symlinkSync,
  existsSync,
  realpathSync,
  readdirSync,
  lstatSync,
} from "node:fs";
import { resolve, join } from "node:path";
import { pathToFileURL } from "node:url";
const [source, generated, runtimePackage, output] = process.argv
  .slice(2)
  .map((resolvePath) => resolve(resolvePath));
const links = join(generated, "node_modules", "@midnight-ntwrk");
mkdirSync(links, { recursive: true });
const link = join(links, "compact-runtime");
if (!existsSync(link)) symlinkSync(runtimePackage, link);
assert.equal(realpathSync(link), realpathSync(runtimePackage));
const r = await import(pathToFileURL(join(runtimePackage, "dist/index.js")));
const { Contract, ledger } = await import(
  pathToFileURL(join(generated, "index.js"))
);
const costs = [],
  originalQuery = r.QueryContext.prototype.query;
r.QueryContext.prototype.query = function (...args) {
  const result = originalQuery.call(this, ...args);
  costs.push({
    gasCost: result.gasCost,
    opTags: args[0].map((op) =>
      typeof op === "string" ? op : Object.keys(op)[0],
    ),
  });
  return result;
};
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
const sum = (queries) =>
  Object.fromEntries(
    ["readTime", "computeTime", "bytesWritten", "bytesDeleted"].map((k) => [
      k,
      queries.reduce((n, q) => n + q.gasCost[k], 0n).toString(),
    ]),
  );
const rows = [];

function run(
  id,
  keyValue,
  enabled,
  selected,
  choices = [true, true],
  fail = null,
) {
  const events = [];
  const callback = (name, index) => (ctx) => {
    events.push([name, ctx.privateState, ctx.ledger.calls]);
    if (name === fail) throw Error(name + " provider refused");
    return [ctx.privateState + 1, choices[index]];
  };
  const contract = new Contract({
      first: callback("first", 0),
      second: callback("second", 1),
    }),
    key = { bytes: new Uint8Array(32) };
  const initial = contract.initialState(
    {
      initialPrivateState: 5,
      initialZswapLocalState: r.emptyZswapLocalState(key),
    },
    "seed",
  );
  const context = r.createCircuitContext(
    r.dummyContractAddress(),
    key,
    initial.currentContractState.data,
    5,
  );
  const start = costs.length;
  let call, error;
  try {
    call = contract.circuits.run(context, keyValue, enabled, selected);
  } catch (e) {
    error = String(e);
  }
  const queries = costs.slice(start),
    expectedEvents = [];
  let failure = null;
  if (!enabled) failure = "disabled";
  else if (selected) {
    expectedEvents.push(["first", 5, 0n]);
    if (fail === "first") failure = "first provider refused";
    else if (!choices[0]) failure = "first denied";
    else {
      expectedEvents.push(["second", 6, 1n]);
      if (fail === "second") failure = "second provider refused";
      else if (!choices[1]) failure = "second denied";
    }
  }
  assert.deepEqual(events, expectedEvents, id);
  const args = [keyValue, enabled, selected];
  const row = {
    id,
    circuit: "run",
    args,
    seed: "seed",
    choices,
    fail,
    events: clean(events),
    queries: clean(queries),
    queryCostSum: sum(queries),
  };
  if (failure) {
    assert.equal(typeof error, "string", id);
    assert.ok(error.includes(failure), id + ": " + error);
    rows.push({ ...row, status: "refused", reason: failure, error });
    return;
  }
  assert.equal(error, undefined, id);
  assert.deepEqual(call.result, [], id);
  assert.equal(call.context.currentPrivateState, selected ? 7 : 5, id);
  // View reads are performed only after snapshotting execution/witness costs.
  const view = ledger(call.context.currentQueryContext.state),
    entries = [...view.entries].sort((a, b) => a[0].localeCompare(b[0]));
  const expected = [["seed", { label: "seed" }]];
  if (selected && keyValue !== "seed")
    expected.push([keyValue, { label: keyValue }]);
  expected.sort((a, b) => a[0].localeCompare(b[0]));
  assert.deepEqual(entries, expected, id);
  assert.equal(view.calls, selected ? 2n : 0n, id);
  assert.equal(view.done, selected, id);
  assert.deepEqual(
    clean(call.proofData.privateTranscriptOutputs).map((x) => x.value),
    selected ? [[[1]], [[1]]] : [],
    id,
  );
  const encoded = r.proofDataIntoSerializedPreimage(
    call.proofData.input,
    call.proofData.output,
    call.proofData.publicTranscript,
    call.proofData.privateTranscriptOutputs,
    "run",
  );
  rows.push({
    ...row,
    status: "success",
    // Expected Rust adapter boundary; not a TS proof-preparation measurement.
    rustPreparationExpectation: selected
      ? "nonempty ledger call"
      : "EmptyTranscript",
    result: [],
    entries: clean(entries),
    calls: selected ? "2" : "0",
    done: selected,
    privateState: call.context.currentPrivateState,
    proofData: clean(call.proofData),
    gasCost: clean(call.gasCost),
    serializedBytes: encoded.length,
  });
}
try {
  for (const [id, key] of [
    ["new", "new"],
    ["overwrite", "seed"],
    ["unicode-nul", "λ\0key"],
    ["empty", ""],
  ])
    run("selected-" + id, key, true, true);
  for (const first of [false, true])
    for (const second of [false, true])
      run(`unselected-${Number(first)}${Number(second)}`, "new", true, false, [
        first,
        second,
      ]);
  for (const fail of ["first", "second"])
    run("suppressed-" + fail, "new", true, false, [true, true], fail);
  for (const selected of [false, true])
    run("disabled-" + selected, "new", false, selected, [true, true], "first");
  run("first-denied", "new", true, true, [false, true]);
  run("second-denied", "new", true, true, [true, false]);
  for (const fail of ["first", "second"])
    run("provider-" + fail, "new", true, true, [true, true], fail);
} finally {
  r.QueryContext.prototype.query = originalQuery;
}
assert.equal(new Set(rows.map((x) => x.id)).size, rows.length);
const files = [];
function add(path) {
  const bytes = readFileSync(path);
  files.push({
    path: realpathSync(path),
    bytes: bytes.length,
    sha256: createHash("sha256").update(bytes).digest("hex"),
  });
}
function tree(path) {
  for (const name of readdirSync(path).sort()) {
    const p = join(path, name),
      s = lstatSync(p);
    if (s.isDirectory()) tree(p);
    else if (s.isFile()) add(p);
  }
}
for (const path of [
  source,
  new URL(import.meta.url),
  join(generated, "index.js"),
  join(generated, "compact-rust-ir.json"),
  join(runtimePackage, "package.json"),
  join(runtimePackage, "package-lock.json"),
])
  add(path);
tree(join(runtimePackage, "dist"));
const wasm = join(
  runtimePackage,
  "node_modules/@midnight-ntwrk/onchain-runtime-v3",
);
tree(wasm);
const report = {
  status: "passed",
  scope:
    "Fresh same-source official TS execution and serialized preimage. One broad Unit root with pure guard, ordered witnesses, repeated helper and Map/Counter/Cell effects; no proof generation, verification or application.",
  versions: {
    node: process.version,
    compactRuntime: JSON.parse(
      readFileSync(join(runtimePackage, "package.json")),
    ).version,
    onchainRuntime: JSON.parse(readFileSync(join(wasm, "package.json")))
      .version,
  },
  files,
  caseCount: rows.length,
  rows,
};
writeFileSync(output, JSON.stringify(report, null, 2) + "\n");
console.log(
  JSON.stringify({
    status: "passed",
    caseCount: rows.length,
    success: rows.filter((x) => x.status === "success").length,
    refused: rows.filter((x) => x.status === "refused").length,
  }),
);
