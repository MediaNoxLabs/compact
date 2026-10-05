// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Pass original call-argument and AssetRegistry generated index.js, corrected runtime, source root.
import { createHash } from "node:crypto";
import { readFile, realpath } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { pathToFileURL } from "node:url";
const [callPath, assetPath, runtimeRoot, sourceRoot] = process.argv.slice(2);
if (!sourceRoot)
  throw new Error("expected two generated contracts, runtime and source root");
for (const path of [callPath, assetPath]) {
  if (
    (await realpath(
      resolve(dirname(path), "node_modules/@midnight-ntwrk/compact-runtime"),
    )) !== (await realpath(runtimeRoot))
  )
    throw new Error("wrong runtime link");
}
const pkg = JSON.parse(
  await readFile(resolve(runtimeRoot, "package.json"), "utf8"),
);
if (pkg.version !== "0.16.101")
  throw new Error("expected corrected runtime 0.16.101");
const runtime = await import(
  pathToFileURL(resolve(runtimeRoot, "dist/index.js"))
);
const call = await import(pathToFileURL(callPath));
const asset = await import(pathToFileURL(assetPath));
const hash = async (p) =>
  createHash("sha256")
    .update(await readFile(p))
    .digest("hex");
const serial = (v) =>
  JSON.parse(
    JSON.stringify(v, (_, x) =>
      typeof x === "bigint"
        ? x.toString()
        : x instanceof Uint8Array
          ? Array.from(x)
          : x,
    ),
  );
const hex32 = (x) =>
  Buffer.from(
    Array.from({ length: 32 }, (_, i) => Number((x >> BigInt(i * 8)) & 255n)),
  ).toString("hex");
const cases = [];
function add(group, name, args, label) {
  const row = {
    id: `${group}/${name}/${label}`,
    group,
    export: name,
    args: serial(args),
  };
  try {
    const result = (group === "call" ? call : asset).pureCircuits[name](
      ...args,
    );
    cases.push({
      ...row,
      ok: true,
      result: typeof result === "bigint" ? hex32(result) : serial(result),
    });
  } catch (e) {
    cases.push({
      ...row,
      ok: false,
      error: e.message,
      errorClass: e.constructor.name,
    });
  }
}
for (const x of [0n, 1n, 1n << 200n]) add("call", "idf", [x], x.toString());
for (const name of ["sumVec", "sumTup"])
  for (const pair of [
    [0n, 1n],
    [1n, 0n],
    [7n, 1n << 200n],
  ])
    add("call", name, [pair], pair.join("-"));
for (const name of [
  "vecFromPureBody",
  "fieldOnlyFromPureBody",
  "tupleIntoVec",
  "vecIntoTuple",
])
  add("call", name, [], "constant");
const max = (1n << 64n) - 1n;
const record = (registered, kind = 1) => ({
  code: new Uint8Array(32).fill(3),
  note: "direct α record",
  provenance: {
    facility: new Uint8Array(32).fill(4),
    registeredAt: registered,
  },
  kind,
  quantity: 5n,
});
for (const [label, enforce, age, registered, current] of [
  ["equal", true, 0n, 100n, 100n],
  ["boundary", true, 20n, 100n, 120n],
  ["expired", true, 19n, 100n, 120n],
  ["unchecked-old", false, 0n, 0n, max],
  ["unchecked-future", false, 0n, 101n, 100n],
  ["future", true, max, 101n, 100n],
  ["max-equal", true, 0n, max, max],
  ["max-span", true, max, 0n, max],
])
  add(
    "asset",
    "assertRecordFreshEnough",
    [{ enforceMaxAge: enforce, maxAge: age }, record(registered), current],
    label,
  );
for (const kind of [0, 1, 2, 3])
  add("asset", "assertRecordClassKnown", [record(17n, kind)], String(kind));
for (const [label, granted, asOf] of [
  ["earlier", 99n, 100n],
  ["equal", 100n, 100n],
  ["future", 101n, 100n],
  ["max-equal", max, max],
  ["max-future", max, max - 1n],
])
  add(
    "asset",
    "assertGrantNotFuture",
    [
      {
        code: new Uint8Array(32).fill(5),
        holder: { bytes: new Uint8Array(32).fill(7) },
        grantedAt: granted,
      },
      asOf,
    ],
    label,
  );
if (
  JSON.stringify(
    [
      ...new Set(cases.filter((x) => x.group === "call").map((x) => x.export)),
    ].sort(),
  ) !== JSON.stringify(Object.keys(call.pureCircuits).sort())
)
  throw new Error("call pure export mismatch");
let queries = [];
const original = runtime.QueryContext.prototype.query;
runtime.QueryContext.prototype.query = function (...args) {
  const result = original.call(this, ...args);
  queries.push({ program: serial(args[0]), gasCost: serial(result.gasCost) });
  return result;
};
const witnessCalls = [];
const contract = new asset.Contract({
  localOperatorKey: ({ privateState }) => {
    witnessCalls.push("localOperatorKey");
    return [
      privateState + 1,
      runtime.hashToCurve(runtime.CompactTypeField, 1n),
    ];
  },
  localAuditorKey: ({ privateState }) => {
    witnessCalls.push("localAuditorKey");
    return [
      privateState + 1,
      runtime.hashToCurve(runtime.CompactTypeField, 2n),
    ];
  },
  currentTimestamp: ({ privateState }) => {
    witnessCalls.push("currentTimestamp");
    return [privateState + 1, 1700000000n];
  },
});
const key = { bytes: new Uint8Array(32) };
const initial = contract.initialState({
  initialPrivateState: 7,
  initialZswapLocalState: runtime.emptyZswapLocalState(key),
});
const stateHex = (s) => Buffer.from(s.serialize()).toString("hex");
const before = stateHex(initial.currentContractState);
const context = runtime.createCircuitContext(
  runtime.dummyContractAddress(),
  key,
  initial.currentContractState.data,
  initial.currentPrivateState,
);
queries = [];
witnessCalls.length = 0;
const output = contract.circuits.close(context);
const success = {
  id: "asset/close/success",
  result: serial(output.result),
  before,
  privateState: output.context.currentPrivateState,
  queries: serial(queries),
  privateTranscript: serial(output.proofData.privateTranscriptOutputs),
  publicTranscript: serial(output.proofData.publicTranscript),
  witnessCalls: [...witnessCalls],
};
initial.currentContractState.data = new runtime.ChargedState(
  output.context.currentQueryContext.state.state,
);
success.after = stateHex(initial.currentContractState);
queries = [];
witnessCalls.length = 0;
let repeat;
try {
  contract.circuits.close(output.context);
  throw new Error("repeat close unexpectedly passed");
} catch (e) {
  if (e.message === "repeat close unexpectedly passed") throw e;
  repeat = {
    id: "asset/close/repeat",
    error: e.message,
    errorClass: e.constructor.name,
    queries: serial(queries),
    witnessCalls: [...witnessCalls],
    privateState: output.context.currentPrivateState,
  };
}
const provenance = { runtimeVersion: pkg.version, sources: {}, runtime: {} };
for (const [name, path] of [
  ["call_arg_declared_type", callPath],
  ["asset_registry_oracle", assetPath],
])
  provenance.sources[name] = {
    sourceSha256: await hash(
      resolve(sourceRoot, `examples/rust_backend/${name}.compact`),
    ),
    generatedIndexSha256: await hash(path),
  };
for (const name of ["index.js", "built-ins.js", "compact-types.js"])
  provenance.runtime[name] = await hash(resolve(runtimeRoot, "dist", name));
process.stdout.write(
  JSON.stringify({ provenance, cases, close: { success, repeat } }, null, 2) +
    "\n",
);
