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
  circuit,
  args,
  expected,
  entries,
  calls = 0n,
  choices = [false, false],
  fail = null,
  expectedEvents = [],
) {
  const events = [];
  const callback = (name, index) => (ctx) => {
    events.push([name, ctx.privateState, ctx.ledger.calls]);
    if (name === fail) throw Error(name + " witness refused");
    return [ctx.privateState + 1, choices[index]];
  };
  const contract = new Contract({
    pick_key: callback("pick_key", 0),
    pick_value: callback("pick_value", 1),
  });
  const key = { bytes: new Uint8Array(32) };
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
    call = contract.circuits[circuit](context, ...args);
  } catch (e) {
    error = String(e);
  }
  const queries = costs.slice(start);
  assert.deepEqual(events, expectedEvents, id);
  const row = {
    id,
    circuit,
    args: clean(args),
    seed: "seed",
    choices,
    fail,
    events: clean(events),
    queries: clean(queries),
    queryCostSum: sum(queries),
  };
  if (expected === "REFUSED") {
    assert.equal(typeof error, "string", id);
    if (fail) assert.match(error, new RegExp(fail + " witness refused"));
    rows.push({ ...row, status: "refused", error });
    return;
  }
  assert.equal(error, undefined, id);
  assert.deepEqual(call.result, expected, id);
  // Snapshot query costs before inspecting the result: these view reads are not circuit execution.
  const view = ledger(call.context.currentQueryContext.state);
  const actualEntries = [...view.table].sort((a, b) =>
    a[0].localeCompare(b[0]),
  );
  assert.deepEqual(
    actualEntries,
    entries.slice().sort((a, b) => a[0].localeCompare(b[0])),
    id,
  );
  assert.equal(view.calls, calls, id);
  assert.equal(call.context.currentPrivateState, 5 + events.length, id);
  assert.deepEqual(
    clean(call.proofData.privateTranscriptOutputs).map((x) => x.value),
    events.map(([name]) => [choices[name === "pick_key" ? 0 : 1] ? [1] : []]),
    id,
  );
  const encoded = r.proofDataIntoSerializedPreimage(
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
    entries: clean(actualEntries),
    calls: String(calls),
    privateState: call.context.currentPrivateState,
    proofData: clean(call.proofData),
    gasCost: clean(call.gasCost),
    serializedBytes: encoded.length,
  });
}
try {
  for (const [key, present] of [
    ["seed", true],
    ["missing", false],
  ]) {
    run("member-" + key, "member", [key], { value: present }, [["seed", 7n]]);
    run("direct-member-" + key, "direct_member", [key], present, [
      ["seed", 7n],
    ]);
  }
  run("lookup-seed", "lookup", ["seed"], { value: 7n }, [["seed", 7n]]);
  run("lookup-missing", "lookup", ["missing"], "REFUSED", []);
  run("direct-lookup-seed", "direct_lookup", ["seed"], 7n, [["seed", 7n]]);
  run("direct-lookup-missing", "direct_lookup", ["missing"], "REFUSED", []);
  for (const [id, key, value] of [
    ["new", "new", 13n],
    ["overwrite", "seed", 11n],
    ["unicode", "λ", 19n],
    ["interior-nul", "a\0b", 23n],
  ])
    run(
      "put-" + id,
      "put",
      [key, value],
      { value },
      key === "seed"
        ? [[key, value]]
        : [
            ["seed", 7n],
            [key, value],
          ],
      1n,
    );
  for (const key of ["seed", "missing"])
    run(
      "remove-" + key,
      "remove",
      [key],
      { value: false },
      key === "seed" ? [] : [["seed", 7n]],
      1n,
    );
  for (const selected of [false, true])
    for (const pickKey of [false, true])
      for (const pickValue of [false, true]) {
        const choices = [pickKey, pickValue],
          entries = [["seed", 7n]],
          events = selected
            ? [
                ["pick_key", 5, 0n],
                ["pick_value", 6, 0n],
              ]
            : [];
        if (selected)
          entries.push([pickKey ? "λ" : "other", pickValue ? 20n : 30n]);
        run(
          `lazy-${Number(selected)}${Number(pickKey)}${Number(pickValue)}`,
          "lazy_put",
          [selected, "λ", "other", 20n, 30n],
          { value: selected && pickKey },
          entries,
          selected ? 1n : 0n,
          choices,
          null,
          events,
        );
      }
  for (const fail of ["pick_key", "pick_value"])
    for (const selected of [false, true]) {
      const events = selected
        ? fail === "pick_key"
          ? [["pick_key", 5, 0n]]
          : [
              ["pick_key", 5, 0n],
              ["pick_value", 6, 0n],
            ]
        : [];
      run(
        `failure-${fail}-${selected}`,
        "lazy_put",
        [selected, "λ", "other", 20n, 30n],
        selected ? "REFUSED" : { value: false },
        [["seed", 7n]],
        0n,
        [true, true],
        fail,
        events,
      );
    }
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
    "Fresh same-source official TS execution and serialized preimage. Scalar5 roots plus direct2 native/TS controls; no proof generation, verification or application.",
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
