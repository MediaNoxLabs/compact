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
  .map((x) => resolve(x));
const links = join(generated, "node_modules", "@midnight-ntwrk");
mkdirSync(links, { recursive: true });
const link = join(links, "compact-runtime");
if (!existsSync(link)) symlinkSync(runtimePackage, link);
assert.equal(realpathSync(link), realpathSync(runtimePackage));
const r = await import(pathToFileURL(join(runtimePackage, "dist/index.js")));
const { Contract, ledger } = await import(
  pathToFileURL(join(generated, "index.js"))
);
const costs = [];
const originalQuery = r.QueryContext.prototype.query;
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
const sum = (queries) =>
  Object.fromEntries(
    ["readTime", "computeTime", "bytesWritten", "bytesDeleted"].map((k) => [
      k,
      queries.reduce((n, q) => n + q.gasCost[k], 0n).toString(),
    ]),
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
const raw = (x) => {
  const a = [];
  while (x > 0n) {
    a.push(Number(x & 255n));
    x >>= 8n;
  }
  return a;
};
const atoms = (x) => x.value.map((a) => [...a]);
const align = (n) => [{ tag: "atom", value: { tag: "bytes", length: n } }];
const field = [{ tag: "atom", value: { tag: "field" } }];
const rows = [];
function execute(circuit, args, value = 0n, fail = null) {
  const events = [];
  const witness = (name) => (ctx) => {
    events.push([name, ctx.privateState, ctx.ledger.calls]);
    if (fail === name) throw Error(name + " witness refused");
    return [ctx.privateState + 1, name === "marker" ? 9n : value];
  };
  const contract = new Contract({
    next129: witness("next129"),
    next248: witness("next248"),
    marker: witness("marker"),
  });
  const key = { bytes: new Uint8Array(32) };
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
  const start = costs.length;
  try {
    return {
      call: contract.circuits[circuit](context, ...args),
      events,
      queries: costs.slice(start),
    };
  } catch (error) {
    return { error: String(error), events, queries: costs.slice(start) };
  }
}
function success(
  circuit,
  args,
  value,
  fail,
  expected,
  events,
  inAlign,
  input,
  outputAtoms,
  outAlign,
) {
  const observed = execute(circuit, args, value, fail);
  assert.equal(observed.error, undefined);
  const { call } = observed;
  assert.deepEqual(call.result, expected);
  assert.deepEqual(observed.events, events);
  assert.equal(ledger(call.context.currentQueryContext.state).calls, 1n);
  assert.equal(call.context.currentPrivateState, 5 + events.length);
  assert.deepEqual(atoms(call.proofData.input), input);
  assert.deepEqual(atoms(call.proofData.output), outputAtoms);
  assert.deepEqual(call.proofData.input.alignment, inAlign);
  assert.deepEqual(call.proofData.output.alignment, outAlign);
  assert.deepEqual(call.gasCost, observed.queries[0].gasCost);
  assert.equal(observed.queries.length, 1 + events.length);
  assert.deepEqual(observed.queries[0].opTags, ["idx", "addi", "ins"]);
  for (const q of observed.queries.slice(1))
    assert.deepEqual(q.opTags, ["dup", "idx", "popeq"]);
  const privateExpected = events.map(([name]) => ({
    value: [raw(name === "marker" ? 9n : value)],
    alignment: name === "marker" ? field : align(name === "next129" ? 17 : 31),
  }));
  assert.deepEqual(
    clean(call.proofData.privateTranscriptOutputs),
    privateExpected,
  );
  const encoded = r.proofDataIntoSerializedPreimage(
    call.proofData.input,
    call.proofData.output,
    call.proofData.publicTranscript,
    call.proofData.privateTranscriptOutputs,
    circuit,
  );
  rows.push({
    circuit,
    args: clean(args),
    value: String(value),
    fail,
    status: "success",
    result: clean(call.result),
    events: clean(events),
    privateState: call.context.currentPrivateState,
    proofData: clean(call.proofData),
    gasCost: clean(call.gasCost),
    queries: clean(observed.queries),
    queryCostSum: sum(observed.queries),
    serializedBytes: encoded.length,
  });
}
function refused(circuit, args, value, fail, events, kind) {
  const actual = execute(circuit, args, value, fail);
  assert.equal(typeof actual.error, "string");
  if (fail) assert.match(actual.error, new RegExp(fail + " witness refused"));
  assert.deepEqual(actual.events, events);
  rows.push({
    circuit,
    args: clean(args),
    value: String(value),
    fail,
    status: "refused",
    refusalKind: kind,
    error: actual.error,
    events: clean(events),
    queries: clean(actual.queries),
    queryCostSum: sum(actual.queries),
  });
}
try {
  for (const bits of [129, 248]) {
    const values = [
      0n,
      1n,
      (1n << 128n) - 1n,
      1n << 128n,
      (1n << BigInt(bits)) - 1n,
    ];
    for (const value of values) {
      success(
        "cast" + bits,
        [value],
        value,
        null,
        { value },
        [],
        align(bits === 129 ? 17 : 31),
        [raw(value)],
        [raw(value)],
        field,
      );
      for (const selected of [false, true]) {
        const expected = selected ? value : 7n;
        const events = selected
          ? [
              [`next${bits}`, 5, 1n],
              ["marker", 6, 1n],
            ]
          : [["marker", 5, 1n]];
        success(
          "witness" + bits,
          [selected],
          value,
          null,
          { value: expected, marker: 9n },
          events,
          align(1),
          [selected ? [1] : []],
          [raw(expected), [9]],
          [...field, ...field],
        );
      }
    }
    for (const fail of [`next${bits}`, "marker"])
      for (const selected of [false, true]) {
        if (!selected && fail !== "marker")
          success(
            "witness" + bits,
            [false],
            1n,
            fail,
            { value: 7n, marker: 9n },
            [["marker", 5, 1n]],
            align(1),
            [[]],
            [[7], [9]],
            [...field, ...field],
          );
        else
          refused(
            "witness" + bits,
            [selected],
            1n,
            fail,
            selected
              ? fail === "marker"
                ? [
                    [`next${bits}`, 5, 1n],
                    ["marker", 6, 1n],
                  ]
                : [[fail, 5, 1n]]
              : [["marker", 5, 1n]],
            "callback",
          );
      }
    for (const invalid of [-1n, 1n << BigInt(bits)])
      refused("cast" + bits, [invalid], 0n, null, [], "public-range");
  }
} finally {
  r.QueryContext.prototype.query = originalQuery;
}
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
    const p = join(path, name);
    const stat = lstatSync(p);
    if (stat.isDirectory()) tree(p);
    else if (stat.isFile()) add(p);
  }
}
add(source);
add(new URL(import.meta.url));
add(join(generated, "index.js"));
add(join(generated, "compact-rust-ir.json"));
add(join(runtimePackage, "package.json"));
add(join(runtimePackage, "package-lock.json"));
tree(join(runtimePackage, "dist"));
const wasm = join(
  runtimePackage,
  "node_modules/@midnight-ntwrk/onchain-runtime-v3",
);
tree(wasm);
const report = {
  status: "passed",
  scope:
    "Fresh same-source official TypeScript execution and serialized proof preimage, no proof generation/verification/application.",
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
  JSON.stringify(
    {
      status: report.status,
      versions: report.versions,
      caseCount: rows.length,
      success: rows.filter((x) => x.status === "success").length,
      refused: rows.filter((x) => x.status === "refused").length,
    },
    null,
    2,
  ),
);
