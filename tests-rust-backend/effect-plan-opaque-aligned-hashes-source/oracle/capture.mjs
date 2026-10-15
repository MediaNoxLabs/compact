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
  realpathSync,
  readdirSync,
  lstatSync,
} from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { createHash } from "node:crypto";
const [source, generated, runtime, output] = process.argv.slice(2);
const links = join(generated, "node_modules/@midnight-ntwrk");
mkdirSync(links, { recursive: true });
const link = join(links, "compact-runtime");
if (!existsSync(link)) symlinkSync(runtime, link);
assert.equal(realpathSync(link), realpathSync(runtime));
const r = await import(pathToFileURL(join(runtime, "dist/index.js")));
const { Contract, ledger } = await import(
  pathToFileURL(join(generated, "index.js"))
);
const costs = [],
  originalQuery = r.QueryContext.prototype.query;
r.QueryContext.prototype.query = function (...args) {
  const result = originalQuery.call(this, ...args);
  costs.push(result.gasCost);
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
      queries.reduce((n, q) => n + q[k], 0n),
    ]),
  );
const rows = [];
for (const op of ["curve", "transient", "commit"])
  for (const type of ["string", "bytes"]) {
    const circuit = `${op}_${type}`,
      values =
        type === "string"
          ? ["", "a", "λ\0key", "a\0"]
          : [[], [0], [1, 0, 2], [1, 0]].map((x) => Uint8Array.from(x));
    for (const [index, input] of values.entries()) {
      const contract = new Contract({}),
        key = { bytes: new Uint8Array(32) };
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
      const start = costs.length,
        call = contract.circuits[circuit](ctx, input),
        queryCostSum = sum(costs.slice(start));
      const descriptor =
        type === "string"
          ? r.CompactTypeOpaqueString
          : r.CompactTypeOpaqueUint8Array;
      let expected;
      if (op === "curve") {
        const point = r.hashToCurve(descriptor, input);
        expected = { x: point.x, y: point.y, value: 0n };
      } else {
        expected = {
          x: 0n,
          y: 0n,
          value:
            op === "transient"
              ? r.transientHash(descriptor, input)
              : r.transientCommit(descriptor, input, 7n),
        };
      }
      assert.deepEqual(call.result, expected);
      assert.equal(ledger(call.context.currentQueryContext.state).calls, 1n);
      assert.equal(call.context.currentPrivateState, 5);
      assert.deepEqual(call.proofData.privateTranscriptOutputs, []);
      const bytes = type === "string" ? new TextEncoder().encode(input) : input;
      rows.push({
        id: `${circuit}-${index}`,
        circuit,
        op,
        type,
        index,
        input: clean(input),
        status: "success",
        result: clean(call.result),
        calls: "1",
        privateState: 5,
        proofData: clean(call.proofData),
        queryCostSum: clean(queryCostSum),
        gasCost: clean(call.gasCost),
        rustObservedExpectation:
          bytes.at(-1) === 0 ? "trailing-zero-refusal" : "success",
      });
    }
  }
const files = [
  source,
  new URL(import.meta.url),
  join(generated, "index.js"),
  join(generated, "compact-rust-ir.json"),
  join(runtime, "package.json"),
  join(runtime, "package-lock.json"),
];
function tree(directory) {
  for (const name of readdirSync(directory).sort()) {
    const path = join(directory, name),
      stat = lstatSync(path);
    if (stat.isDirectory()) tree(path);
    else if (stat.isFile()) files.push(path);
  }
}
tree(join(runtime, "dist"));
tree(join(runtime, "node_modules/@midnight-ntwrk/onchain-runtime-v3"));
const report = {
  status: "passed",
  scope:
    "Fresh genuine default-v2 TS execution; no proof. Rust observed expectations are labels, not measured TS refusals. PersistentHash opaque inputs are excluded by official source semantics.",
  versions: {
    node: process.version,
    compactRuntime: JSON.parse(readFileSync(join(runtime, "package.json")))
      .version,
  },
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
    expectedRustObservedSuccesses: rows.filter(
      (x) => x.rustObservedExpectation === "success",
    ).length,
    expectedRustObservedRefusals: rows.filter(
      (x) => x.rustObservedExpectation !== "success",
    ).length,
  }),
);
