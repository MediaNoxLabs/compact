// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
// Unchanged generated TS: literal, assert, ternary contract/index.js, corrected runtime, source root.
import { createHash } from "node:crypto";
import { readFile, realpath } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { pathToFileURL } from "node:url";
const [literalPath, assertPath, ternaryPath, runtimeRoot, sourceRoot] =
  process.argv.slice(2);
if (!sourceRoot)
  throw new Error(
    "expected three contract paths, corrected runtime root, source root",
  );
for (const p of [literalPath, assertPath, ternaryPath]) {
  if (
    (await realpath(
      resolve(dirname(p), "node_modules/@midnight-ntwrk/compact-runtime"),
    )) !== (await realpath(runtimeRoot))
  )
    throw new Error("wrong contract runtime");
}
const runtimePackage = JSON.parse(
  await readFile(resolve(runtimeRoot, "package.json"), "utf8"),
);
if (runtimePackage.version !== "0.16.101")
  throw new Error("expected corrected runtime 0.16.101");
const runtime = await import(
  pathToFileURL(resolve(runtimeRoot, "dist/index.js")).href
);
const { pureCircuits } = await import(pathToFileURL(literalPath).href);
const { Contract: Assert } = await import(pathToFileURL(assertPath).href);
const { Contract: Ternary, ledger: ternaryLedger } = await import(
  pathToFileURL(ternaryPath).href
);
const hash = async (p) =>
  createHash("sha256")
    .update(await readFile(p))
    .digest("hex");
const fieldHex = (value) => {
  const bytes = new Uint8Array(32);
  for (let i = 0; i < 32; i++)
    bytes[i] = Number((value >> BigInt(8 * i)) & 255n);
  return Buffer.from(bytes).toString("hex");
};
function encode(value) {
  if (typeof value === "bigint") return fieldHex(value);
  if (value instanceof Uint8Array) return Buffer.from(value).toString("hex");
  if (Array.isArray(value)) return value.map(encode);
  if (value && typeof value === "object")
    return Object.fromEntries(
      Object.entries(value).map(([k, v]) => [k, encode(v)]),
    );
  return value;
}
const jsonArgs = (v) =>
  JSON.parse(
    JSON.stringify(v, (_, x) => (typeof x === "bigint" ? x.toString() : x)),
  );
const cases = [];
function add(name, args = [], label = "constant") {
  cases.push({
    id: `${name}/${label}`,
    export: name,
    args: jsonArgs(args),
    result: encode(pureCircuits[name](...args)),
  });
}
const constants = [
  "retFieldLiteral",
  "constThenCall",
  "structLiteral",
  "someFieldLiteral",
  "hashZeroLiteral",
  "nativeArgLiteral",
  "retU128FieldLiteral",
  "retHugeFieldLiteral",
  "constHugeFieldLiteral",
  "callArgHugeFieldLiteral",
  "structMemberHugeFieldLiteral",
  "vectorEltHugeFieldLiteral",
  "hashConstVectorArg",
  "sameTypeVec",
  "hashCallVectorArg",
  "hashDefaultVectorArg",
  "callArgFieldOnlyLiteral",
  "constFieldOnlyLiteral",
  "retFieldOnlyLiteral",
  "callArgMaxUnsignedPlusOne",
  "subgroupCheck",
];
for (const name of constants) add(name);
for (const name of [
  "idf",
  "addFieldLiteral",
  "addHugeFieldLiteral",
  "litLhsFieldLiteral",
  "subFieldLiteral",
  "mulHugeFieldLiteral",
  "nestedFieldArith",
  "addFieldOnlyLiteral",
])
  for (const x of [0n, 1n, (1n << 128n) - 1n, 1n << 200n])
    add(name, [x], x.toString());
for (const x of [0n, 255n])
  for (const name of [
    "uintToField",
    "hashUintVarRefElem",
    "hashNestedUintVarRefElem",
  ])
    add(name, [x], x.toString());
for (const x of [0n, (1n << 64n) - 1n, 1n << 64n, (1n << 128n) - 1n])
  add("u128Rung", [x], x.toString());
for (const x of [0n, (1n << 32n) - 1n]) add("vectorEltWise", [x], x.toString());
const fieldOnly =
  819310549611346726241370945440405716213240158234039660170669895299022906775n;
for (const [name, expected] of [
  ["cmpHugeFieldLiteral", 1n << 200n],
  ["cmpFieldOnlyLiteral", fieldOnly],
])
  for (const x of [0n, expected, expected + 1n]) add(name, [x], x.toString());
for (const [x, u] of [
  [0n, 0n],
  [1n, 255n],
  [1n << 200n, 255n],
])
  add("addUintFieldOperand", [x, u], `${x}-${u}`);
for (const values of [
  [0n, 0n],
  [255n, 1n],
  [1n, 255n],
]) {
  for (const name of [
    "hashVarRefVectorArg",
    "hashNestedVarRefVectorArg",
    "hashSameTypeVectorArg",
    "hashSameTypeNestedArg",
  ])
    add(name, [values], values.join("-"));
  add("hashStructFieldVectorArg", [{ v: values }], values.join("-"));
}
for (const scalar of [1n, 2n]) {
  const point = runtime.ecMulGenerator(scalar);
  cases.push({
    id: `nativeArgFieldOnlyLiteral/generator-${scalar}`,
    export: "nativeArgFieldOnlyLiteral",
    args: [scalar.toString()],
    point: encode(point),
    result: encode(pureCircuits.nativeArgFieldOnlyLiteral(point)),
  });
}
const exports = Object.keys(pureCircuits).sort();
if (
  JSON.stringify([...new Set(cases.map((c) => c.export))].sort()) !==
  JSON.stringify(exports)
)
  throw new Error("export case set mismatch");
let queries = [];
const originalQuery = runtime.QueryContext.prototype.query;
runtime.QueryContext.prototype.query = function (...args) {
  const result = originalQuery.call(this, ...args);
  queries.push({ gasCost: jsonArgs(result.gasCost) });
  return result;
};
const key = { bytes: new Uint8Array(32) };
const stateHex = (s) => Buffer.from(s.serialize()).toString("hex");
function capture(contract, name, args, constructorArgs = []) {
  const initial = contract.initialState(
    {
      initialPrivateState: null,
      initialZswapLocalState: runtime.emptyZswapLocalState(key),
    },
    ...constructorArgs,
  );
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
  return {
    export: name,
    args: jsonArgs(args),
    result: jsonArgs(out.result),
    before: stateHex(initial.currentContractState),
    after: stateHex(after),
    queries: querySnapshot,
    reportedGas: jsonArgs(out.gasCost),
    publicTranscript: out.proofData.publicTranscript,
    privateTranscript: out.proofData.privateTranscriptOutputs,
    ledgerVector:
      name === "walkerVectorElement"
        ? encode(ternaryLedger(after.data).vecCell)
        : undefined,
  };
}
const sources = [
  "literal_coercion_oracle",
  "assert_parity_oracle",
  "ternary_cond_oracle",
];
const paths = [literalPath, assertPath, ternaryPath];
const provenance = {
  runtimeVersion: runtimePackage.version,
  sources: {},
  runtime: {},
};
for (let i = 0; i < sources.length; i++)
  provenance.sources[sources[i]] = {
    sourceSha256: await hash(
      resolve(sourceRoot, `examples/rust_backend/${sources[i]}.compact`),
    ),
    generatedIndexSha256: await hash(paths[i]),
  };
for (const name of ["index.js", "built-ins.js", "compact-types.js"])
  provenance.runtime[name] = await hash(resolve(runtimeRoot, "dist", name));
const ternary = new Ternary({
  echoField() {
    throw new Error("unexpected witness");
  },
});
const stateful = [
  capture(new Assert({}), "ping", []),
  capture(ternary, "walkerVectorElement", [false], [true, true, 111n]),
  capture(ternary, "walkerVectorElement", [true], [true, true, 111n]),
];
process.stdout.write(
  JSON.stringify(
    { provenance, literal: { exports, cases }, stateful },
    (_, v) =>
      typeof v === "bigint"
        ? v.toString()
        : v instanceof Uint8Array
          ? Array.from(v)
          : v,
    2,
  ) + "\n",
);
