// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Four unchanged generated TS modules, corrected runtime, source root.
import { createHash } from "node:crypto";
import { readFile, realpath } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { pathToFileURL } from "node:url";
const [mapPath, nestedPath, witnessPath, setPath, runtimeRoot, sourceRoot] =
  process.argv.slice(2);
if (!sourceRoot)
  throw new Error("expected four contract paths, runtime root, source root");
const paths = [mapPath, nestedPath, witnessPath, setPath];
const names = [
  "map_oracle",
  "nested_map_oracle",
  "witnesses_oracle",
  "set_oracle",
];
for (const p of paths) {
  if (
    (await realpath(
      resolve(dirname(p), "node_modules/@midnight-ntwrk/compact-runtime"),
    )) !== (await realpath(runtimeRoot))
  )
    throw new Error("wrong runtime");
}
const runtimePackage = JSON.parse(
  await readFile(resolve(runtimeRoot, "package.json"), "utf8"),
);
if (runtimePackage.version !== "0.16.101")
  throw new Error("expected corrected runtime 0.16.101");
const runtime = await import(
  pathToFileURL(resolve(runtimeRoot, "dist/index.js")).href
);
const modules = await Promise.all(
  paths.map((p) => import(pathToFileURL(p).href)),
);
const hash = async (p) =>
  createHash("sha256")
    .update(await readFile(p))
    .digest("hex");
const json = (value) =>
  JSON.parse(
    JSON.stringify(value, (_, v) =>
      typeof v === "bigint"
        ? v.toString()
        : v instanceof Uint8Array
          ? Array.from(v)
          : v,
    ),
  );
const hex = (s) => Buffer.from(s.serialize()).toString("hex");
const key = { bytes: new Uint8Array(32) };
let queries = [];
const originalQuery = runtime.QueryContext.prototype.query;
runtime.QueryContext.prototype.query = function (...args) {
  const result = originalQuery.call(this, ...args);
  queries.push({ gasCost: json(result.gasCost) });
  return result;
};
const cases = [];
function chain(
  module,
  source,
  name,
  steps,
  witnesses = {},
  privateState = null,
) {
  const contract = new module.Contract(witnesses);
  let initial = contract.initialState({
    initialPrivateState: privateState,
    initialZswapLocalState: runtime.emptyZswapLocalState(key),
  });
  for (const [label, args] of steps) {
    const context = runtime.createCircuitContext(
      runtime.dummyContractAddress(),
      key,
      initial.currentContractState.data,
      initial.currentPrivateState,
    );
    queries = [];
    const out = contract.circuits[name](context, ...args);
    const querySnapshot = queries;
    queries = [];
    const after = new runtime.ContractState();
    after.data = new runtime.ChargedState(
      out.context.currentQueryContext.state.state,
    );
    for (const op of initial.currentContractState.operations())
      after.setOperation(op, initial.currentContractState.operation(op));
    after.maintenanceAuthority =
      initial.currentContractState.maintenanceAuthority;
    after.balance = initial.currentContractState.balance;
    cases.push({
      id: `${source}/${name}/${label}`,
      source,
      export: name,
      args: json(args),
      result: json(out.result),
      before: hex(initial.currentContractState),
      after: hex(after),
      privateBefore: initial.currentPrivateState,
      privateAfter: out.context.currentPrivateState,
      queries: querySnapshot,
      reportedGas: json(out.gasCost),
      publicTranscript: json(out.proofData.publicTranscript),
      privateTranscript: out.proofData.privateTranscriptOutputs.map((v) => ({
        valueAtoms: v.value.map((a) => Array.from(a)),
        alignment: v.alignment,
      })),
    });
    initial = {
      currentContractState: after,
      currentPrivateState: out.context.currentPrivateState,
    };
  }
}
chain(modules[0], names[0], "put", [
  ["insert", [7n, 9n]],
  ["replace", [7n, 11n]],
  ["distinct", [8n, 13n]],
]);
chain(modules[1], names[1], "ping", [
  ["initial", []],
  ["repeat", []],
]);
for (const value of [42n, 0n]) {
  chain(
    modules[2],
    names[2],
    "pull",
    [[`witness-${value}`, []]],
    {
      fetch_field: (context) => {
        if (context.privateState !== 7 || context.ledger.v !== 0n)
          throw new Error("wrong witness context");
        return [8, value];
      },
    },
    7,
  );
}
chain(modules[3], names[3], "check", [
  ["seven", [7n]],
  ["eight", [8n]],
]);
const provenance = {
  runtimeVersion: runtimePackage.version,
  sources: {},
  runtime: {},
};
for (let i = 0; i < paths.length; i++)
  provenance.sources[names[i]] = {
    sourceSha256: await hash(
      resolve(sourceRoot, `examples/rust_backend/${names[i]}.compact`),
    ),
    generatedIndexSha256: await hash(paths[i]),
  };
for (const name of ["index.js", "built-ins.js", "compact-types.js"])
  provenance.runtime[name] = await hash(resolve(runtimeRoot, "dist", name));
process.stdout.write(JSON.stringify({ provenance, cases }, null, 2) + "\n");
