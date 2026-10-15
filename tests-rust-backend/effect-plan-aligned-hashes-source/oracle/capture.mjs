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
  readdirSync,
  lstatSync,
} from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
const [source, generated, runtime, output] = process.argv.slice(2);
const links = join(generated, "node_modules/@midnight-ntwrk");
mkdirSync(links, { recursive: true });
if (!existsSync(join(links, "compact-runtime")))
  symlinkSync(runtime, join(links, "compact-runtime"));
const r = await import(pathToFileURL(join(runtime, "dist/index.js")));
const queries = [];
const originalQuery = r.QueryContext.prototype.query;
r.QueryContext.prototype.query = function (...args) {
  const result = originalQuery.call(this, ...args);
  queries.push(result.gasCost);
  return result;
};
const sum = (costs) =>
  Object.fromEntries(
    ["readTime", "computeTime", "bytesWritten", "bytesDeleted"].map((k) => [
      k,
      costs.reduce((n, q) => n + q[k], 0n),
    ]),
  );
const { Contract, ledger } = await import(
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
const sha = (p) => createHash("sha256").update(readFileSync(p)).digest("hex");
const answers = new Map(
  [0n, 1n, 2n, 17n].map((v) => [
    v,
    {
      point: r.hashToCurve(r.CompactTypeField, v),
      digest: r.persistentHash(r.CompactTypeField, v),
    },
  ]),
);
const rows = [];
function run({
  id,
  circuit,
  value = 1n,
  other = 2n,
  selected = true,
  fail = null,
  mismatch = null,
}) {
  const events = [];
  const callbacks = Object.fromEntries(
    ["curveInput", "hashInput", "expectedPoint", "expectedDigest"].map(
      (name) => [
        name,
        (ctx) => {
          events.push([name, ctx.privateState, ctx.ledger.calls]);
          if (fail === name) throw Error(`${name} provider refused`);
          const answer =
            name === "curveInput"
              ? value
              : name === "hashInput"
                ? other
                : name === "expectedPoint"
                  ? answers.get(mismatch === name ? 17n : value).point
                  : answers.get(mismatch === name ? 17n : value).digest;
          return [ctx.privateState + 1, answer];
        },
      ],
    ),
  );
  const contract = new Contract(callbacks),
    key = { bytes: new Uint8Array(32) };
  const initial = contract.initialState({
    initialPrivateState: 5,
    initialZswapLocalState: r.emptyZswapLocalState(key),
  });
  const context = r.createCircuitContext(
    r.dummyContractAddress(),
    key,
    initial.currentContractState.data,
    5,
  );
  const isClose = circuit.startsWith("close"),
    args = [isClose ? value : selected];
  const names = isClose
    ? circuit === "close"
      ? ["expectedPoint", "expectedDigest"]
      : ["expectedDigest"]
    : selected
      ? circuit === "hashes"
        ? ["curveInput", "hashInput"]
        : ["hashInput"]
      : [];
  const expectedEvents = [];
  let reason = null;
  for (const name of names) {
    expectedEvents.push([name, 5 + expectedEvents.length, isClose ? 0n : 1n]);
    if (fail === name) {
      reason = `${name} provider refused`;
      break;
    }
    if (mismatch === name) {
      reason = name === "expectedPoint" ? "point mismatch" : "digest mismatch";
      break;
    }
  }
  const start = queries.length;
  let result, error;
  try {
    result = contract.circuits[circuit](context, ...args);
  } catch (e) {
    error = String(e);
  }
  const queryCostSum = sum(queries.slice(start));
  assert.deepEqual(events, expectedEvents, id);
  const row = {
    id,
    circuit,
    value,
    other,
    selected,
    fail,
    mismatch,
    args,
    events,
    queryCostSum,
  };
  if (reason) {
    assert.ok(error?.includes(reason), `${id}: ${error}`);
    rows.push({ ...row, status: "refused", reason, error });
    return;
  }
  assert.equal(error, undefined, id);
  const expected = isClose
    ? []
    : {
        x: selected && circuit === "hashes" ? answers.get(value).point.x : 0n,
        digest: selected ? answers.get(other).digest : new Uint8Array(32),
      };
  assert.deepEqual(result.result, expected, id);
  assert.equal(ledger(result.context.currentQueryContext.state).calls, 1n, id);
  assert.equal(result.context.currentPrivateState, 5 + events.length, id);
  rows.push({
    ...row,
    status: "success",
    result: result.result,
    privateState: result.context.currentPrivateState,
    calls: 1n,
    proofData: result.proofData,
    gasCost: result.gasCost,
  });
}
for (const value of [0n, 1n, 2n])
  for (const circuit of ["close", "close_hash", "hashes", "hash_only"])
    run({ id: `${circuit}-${value}`, circuit, value, other: value });
run({ id: "hashes-ordered-distinct", circuit: "hashes", value: 1n, other: 2n });
for (const circuit of ["hashes", "hash_only"]) {
  run({ id: `${circuit}-lazy`, circuit, selected: false, fail: "hashInput" });
  run({ id: `${circuit}-hash-refusal`, circuit, fail: "hashInput" });
}
run({ id: "hashes-curve-refusal", circuit: "hashes", fail: "curveInput" });
for (const name of ["expectedPoint", "expectedDigest"]) {
  run({ id: `close-${name}-refusal`, circuit: "close", fail: name });
  run({ id: `close-${name}-mismatch`, circuit: "close", mismatch: name });
}
for (const kind of ["refusal", "mismatch"])
  run({
    id: `close_hash-${kind}`,
    circuit: "close_hash",
    ...(kind === "refusal"
      ? { fail: "expectedDigest" }
      : { mismatch: "expectedDigest" }),
  });
const files = [
  source,
  new URL(import.meta.url),
  join(generated, "index.js"),
  join(generated, "compact-rust-ir.json"),
  join(runtime, "package.json"),
  join(runtime, "package-lock.json"),
];
function bindTree(directory) {
  for (const entry of readdirSync(directory).sort()) {
    const path = join(directory, entry);
    const stat = lstatSync(path);
    if (stat.isDirectory()) bindTree(path);
    else if (stat.isFile()) files.push(path);
  }
}
bindTree(join(runtime, "dist"));
bindTree(join(runtime, "node_modules/@midnight-ntwrk/onchain-runtime-v3"));
const report = {
  status: "passed",
  scope:
    "Fresh same-source official TypeScript execution. Hash answers independently computed via official aligned APIs; not published cryptographic vectors. No proof execution in this capture.",
  node: process.version,
  files: Object.fromEntries(files.map((p) => [p, sha(p)])),
  knownAnswers: [...answers].map(([input, answer]) => ({ input, ...answer })),
  caseCount: rows.length,
  rows,
};
writeFileSync(output, JSON.stringify(clean(report), null, 2) + "\n");
console.log(
  JSON.stringify({
    status: "passed",
    cases: rows.length,
    success: rows.filter((x) => x.status === "success").length,
    refused: rows.filter((x) => x.status === "refused").length,
  }),
);
