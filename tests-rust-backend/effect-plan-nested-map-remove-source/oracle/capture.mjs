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
    program: clean(args[0]),
    effects: clean(result.context.effects),
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
const uint64 = new r.CompactTypeUnsignedInteger((1n << 64n) - 1n, 8);
const aligned = (type, value) => ({ value: type.toValue(value), alignment: type.alignment() });
const field = value => aligned(r.CompactTypeField, value);
const child = value => r.StateValue.newMap(new r.StateMap().insert(
  field(11n), r.StateValue.newCell(aligned(uint64, value)),
));
function supplied(present) {
  let outer = new r.StateMap().insert(field(2n), child(90n));
  if (present) outer = outer.insert(field(1n), child(10n));
  return r.StateValue.newArray().arrayPush(r.StateValue.newMap(outer));
}
// Official StateMap/StateValue setup: selected key1 child{11:10}, sibling
// key2 child{11:90}. Generated nested scalar insertion is not claimed.
try {
  for (const present of [true, false]) {
    const id = present ? "present" : "absent";
    const contract = new Contract({});
    const coinKey = { bytes: new Uint8Array(32) };
    const initial = supplied(present);
    const expected = supplied(false);
    const context = r.createCircuitContext(
      r.dummyContractAddress(), coinKey, new r.ChargedState(initial), 5,
    );
    const start = costs.length;
    const call = contract.circuits.remove(context, 1n);
    const queries = costs.slice(start);
    assert.deepEqual(call.result, [], id);
    assert.equal(call.context.currentPrivateState, 5, id);
    assert.deepEqual(call.proofData.privateTranscriptOutputs, [], id);
    assert.deepEqual(clean(call.context.currentQueryContext.state.state.encode()), clean(expected.encode()), id);
    const outer = call.context.currentQueryContext.state.state.asArray()[0].asMap();
    assert.equal(outer.get(field(1n)), undefined, id);
    assert.deepEqual(clean(outer.get(field(2n)).encode()), clean(child(90n).encode()), id);
    assert.equal(outer.keys().length, 1, id);
    const encoded = r.proofDataIntoSerializedPreimage(
      call.proofData.input, call.proofData.output, call.proofData.publicTranscript,
      call.proofData.privateTranscriptOutputs, "remove",
    );
    rows.push({
      id, circuit: "remove", args: ["1"], present, status: "success",
      suppliedEntries: present ? [["1", [["11", "10"]]], ["2", [["11", "90"]]]] : [["2", [["11", "90"]]]],
      remainingEntries: [["2", [["11", "90"]]]],
      result: call.result, privateState: call.context.currentPrivateState,
      initialState: clean(initial.encode()), finalState: clean(expected.encode()),
      effects: clean(call.context.currentQueryContext.effects),
      proofData: clean(call.proofData), gasCost: clean(call.gasCost),
      queries: clean(queries), queryCostSum: sum(queries), serializedBytes: encoded.length,
    });
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
    "Fresh same-source official TS execution and serialized preimage for present/absent key-only nested Map removal. Populated children supplied through official ledger StateMap/StateValue; full sibling preserved. No generated nested insertion/lookup codec, proof generation, verification or application.",
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
