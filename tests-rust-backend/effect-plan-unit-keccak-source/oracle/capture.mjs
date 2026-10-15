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

const vectors = [
  {
    name: "zero",
    input: new Uint8Array(32),
    hex: "c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470",
  },
  {
    name: "ff",
    input: new Uint8Array(32).fill(255),
    hex: "a9c584056064687e149968cbab758a3376d22aedc6a55823d1b3ecbee81b8fb9",
  },
  {
    name: "ramp",
    input: Uint8Array.from({ length: 32 }, (_, i) => i),
    hex: "8ae1aa597fa146ebd3aa2ceddf360668dea5e526567e92b0321816a4e895bd2d",
  },
];
for (const vector of vectors) {
  vector.digest = Uint8Array.from(Buffer.from(vector.hex, "hex"));
  assert.deepEqual(
    r.keccak256(new r.CompactTypeBytes(32), vector.input),
    vector.digest,
    vector.name + " official TS known answer",
  );
}
function run(
  id,
  circuit,
  vector,
  selected = true,
  mismatch = false,
  authorized = true,
  fail = null,
) {
  const events = [];
  const expectedHash = mismatch ? new Uint8Array(32) : vector.digest;
  const callback = (name) => (ctx) => {
    events.push([name, ctx.privateState, ctx.ledger.active]);
    if (name === fail) throw Error(name + " provider refused");
    return [
      ctx.privateState + 1,
      name === "expectedHash"
        ? expectedHash
        : name === "digestPayload"
          ? vector.input
          : authorized,
    ];
  };
  const contract = new Contract({
      expectedHash: callback("expectedHash"),
      authorized: callback("authorized"),
      digestPayload: callback("digestPayload"),
    }),
    key = { bytes: new Uint8Array(32) };
  const initial = contract.initialState({
      initialPrivateState: 5,
      initialZswapLocalState: r.emptyZswapLocalState(key),
    }),
    context = r.createCircuitContext(
      r.dummyContractAddress(),
      key,
      initial.currentContractState.data,
      5,
    );
  const args =
    circuit === "digest"
      ? [selected]
      : circuit === "close"
        ? [vector.input]
        : [vector.input, selected];
  const start = costs.length;
  let call, error;
  try {
    call = contract.circuits[circuit](context, ...args);
  } catch (e) {
    error = String(e);
  }
  const queries = costs.slice(start);
  const expectedEvents = [];
  let failure = null;
  if (circuit !== "lazy_close" || selected) {
    const first = circuit === "digest" ? "digestPayload" : "expectedHash";
    if (circuit !== "digest" || selected) {
      expectedEvents.push([first, 5, true]);
      if (fail === first) failure = first + " provider refused";
      else if (circuit !== "digest" && mismatch) failure = "hash mismatch";
    }
    if (!failure) {
      expectedEvents.push(["authorized", 5 + expectedEvents.length, true]);
      if (fail === "authorized") failure = "authorized provider refused";
      else if (!authorized) failure = "not authorized";
    }
  }
  assert.deepEqual(events, expectedEvents, id);
  const row = {
    id,
    circuit,
    vector: vector.name,
    args: clean(args),
    selected,
    mismatch,
    authorized,
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
  const expected =
    circuit === "digest"
      ? { value: selected ? vector.digest : new Uint8Array(32) }
      : [];
  assert.deepEqual(call.result, expected, id);
  const active = circuit === "lazy_close" && !selected;
  assert.equal(
    ledger(call.context.currentQueryContext.state).active,
    active,
    id,
  );
  assert.equal(call.context.currentPrivateState, 5 + events.length, id);
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
    active,
    privateState: call.context.currentPrivateState,
    proofData: clean(call.proofData),
    gasCost: clean(call.gasCost),
    serializedBytes: encoded.length,
    rustPreparationExpectation: active
      ? "EmptyTranscript"
      : "nonempty ledger call",
  });
}
try {
  for (const v of vectors) {
    run("close-" + v.name, "close", v);
    run("lazy-true-" + v.name, "lazy_close", v);
    run("digest-true-" + v.name, "digest", v);
    run("digest-false-" + v.name, "digest", v, false);
  }
  for (const [kind, mismatch, authorized, fail] of [
    ["hash-refusal", false, true, "expectedHash"],
    ["auth-refusal", false, true, "authorized"],
    ["mismatch", true, true, null],
    ["denied", false, false, null],
  ]) {
    run(
      "lazy-false-" + kind,
      "lazy_close",
      vectors[0],
      false,
      mismatch,
      authorized,
      fail,
    );
    run("close-" + kind, "close", vectors[0], true, mismatch, authorized, fail);
    run(
      "lazy-true-" + kind,
      "lazy_close",
      vectors[0],
      true,
      mismatch,
      authorized,
      fail,
    );
  }
  run(
    "digest-suppressed-payload",
    "digest",
    vectors[0],
    false,
    false,
    true,
    "digestPayload",
  );
  run(
    "digest-payload-refusal",
    "digest",
    vectors[0],
    true,
    false,
    true,
    "digestPayload",
  );
  for (const selected of [false, true]) {
    run(
      "digest-auth-refusal-" + selected,
      "digest",
      vectors[0],
      selected,
      false,
      true,
      "authorized",
    );
    run(
      "digest-denied-" + selected,
      "digest",
      vectors[0],
      selected,
      false,
      false,
      null,
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
    "Fresh same-source official TS execution and serialized preimage. ZKIR-v3 source frontend and native TS execution. Two old-positive Unit controls plus one checked Digest expansion. Default ZKIR-v2 refuses Keccak; no Keccak proof generation/verification/application.",
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
  knownAnswers: vectors.map((v) => ({
    name: v.name,
    inputHex: Buffer.from(v.input).toString("hex"),
    digestHex: v.hex,
  })),
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
