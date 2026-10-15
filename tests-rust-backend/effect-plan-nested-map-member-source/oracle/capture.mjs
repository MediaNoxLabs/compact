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
// The populated state is supplied through official ledger StateMap/StateValue APIs.
// It is NOT emitted by a generated insertDefault constructor.
try {
  for (const supplied of [false, true])
    for (const keyValue of [1n, 2n]) {
      const contract = new Contract({}),
        key = { bytes: new Uint8Array(32) };
      const initial = contract.initialState({
        initialPrivateState: 5,
        initialZswapLocalState: r.emptyZswapLocalState(key),
      });
      let charged = initial.currentContractState.data;
      if (supplied) {
        const outer = new r.StateMap().insert(
          {
            value: r.CompactTypeField.toValue(1n),
            alignment: r.CompactTypeField.alignment(),
          },
          r.StateValue.newMap(new r.StateMap()),
        );
        charged = new r.ChargedState(
          r.StateValue.newArray().arrayPush(r.StateValue.newMap(outer)),
        );
      }
      const context = r.createCircuitContext(
        r.dummyContractAddress(),
        key,
        charged,
        5,
      );
      const start = costs.length,
        call = contract.circuits.member(context, keyValue),
        queries = costs.slice(start);
      assert.deepEqual(call.result, { value: supplied && keyValue === 1n });
      assert.equal(call.context.currentPrivateState, 5);
      assert.deepEqual(call.proofData.privateTranscriptOutputs, []);
      const encoded = r.proofDataIntoSerializedPreimage(
        call.proofData.input,
        call.proofData.output,
        call.proofData.publicTranscript,
        call.proofData.privateTranscriptOutputs,
        "member",
      );
      rows.push({
        id: `${supplied ? "supplied" : "generated-empty"}-${keyValue}`,
        supplied,
        circuit: "member",
        args: [String(keyValue)],
        status: "success",
        result: call.result,
        privateState: 5,
        proofData: clean(call.proofData),
        gasCost: clean(call.gasCost),
        queries: clean(queries),
        queryCostSum: sum(queries),
        serializedBytes: encoded.length,
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
    "Fresh same-source official TS execution and serialized preimage. Nested key-only membership in generated empty and explicitly supplied official ledger states; no seeded constructor, nested scalar codec, proof generation, verification or application.",
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
