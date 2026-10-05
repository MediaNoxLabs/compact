// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Pass unchanged generated contract/index.js, corrected runtime root, Compact source.
import { createHash } from "node:crypto";
import { readFile, realpath } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { pathToFileURL } from "node:url";
const [contractPath, runtimeRoot, sourcePath] = process.argv.slice(2);
if (!sourcePath)
  throw new Error("expected generated contract, runtime, source");
if (
  (await realpath(
    resolve(
      dirname(contractPath),
      "node_modules/@midnight-ntwrk/compact-runtime",
    ),
  )) !== (await realpath(runtimeRoot))
)
  throw new Error("incorrect runtime link");
const pkg = JSON.parse(
  await readFile(resolve(runtimeRoot, "package.json"), "utf8"),
);
if (pkg.version !== "0.16.101")
  throw new Error("expected corrected runtime0.16.101");
const { pureCircuits } = await import(pathToFileURL(contractPath).href);
const hash = async (p) =>
  createHash("sha256")
    .update(await readFile(p))
    .digest("hex");
function encode(v) {
  if (typeof v === "bigint") {
    const b = new Uint8Array(32);
    for (let i = 0; i < 32; i++) b[i] = Number((v >> BigInt(i * 8)) & 255n);
    return Buffer.from(b).toString("hex");
  }
  if (v instanceof Uint8Array) return Buffer.from(v).toString("hex");
  if (Array.isArray(v)) return v.map(encode);
  if (v && typeof v === "object")
    return Object.fromEntries(
      Object.entries(v).map(([k, x]) => [k, encode(x)]),
    );
  return v;
}
const cases = [];
function add(name, args) {
  const encodedArgs = JSON.parse(
    JSON.stringify(args, (_, v) => (typeof v === "bigint" ? v.toString() : v)),
  );
  const id = `${name}/${JSON.stringify(encodedArgs)}`;
  try {
    cases.push({
      id,
      export: name,
      args: encodedArgs,
      ok: true,
      result: encode(pureCircuits[name](...args)),
    });
  } catch (e) {
    cases.push({
      id,
      export: name,
      args: encodedArgs,
      ok: false,
      error: e.message,
      errorClass: e.constructor.name,
    });
  }
}
for (const x of [0n, 1n, 1n << 200n]) add("idf", [x]);
for (const c of [false, true]) {
  for (const name of [
    "constAnnotatedBothLiteral",
    "structMember",
    "vectorElement",
    "enumValued",
    "literalAboveI32",
    "literalAboveU64",
    "walkerReturnTail",
    "walkerCallCtor",
    "nativeVectorElemTernary",
  ])
    add(name, [c]);
  for (const n of [0n, 1n, 5n, 255n]) add("constUnannotatedSeqLifted", [c, n]);
  for (const n of [0n, 255n]) {
    add("returnTailMixed", [c, n]);
    add("arithOperand", [c, n]);
  }
  for (const n of [0n, 1n, 2n, 3n, 255n]) add("assertArg", [c, n]);
  for (const n of [0n, 1n, 255n]) add("cmpOperand", [c, n]);
  for (const d of [false, true]) add("returnTailNested", [c, d]);
  for (const name of [
    "callArgPure",
    "callArgCtor",
    "fieldTypedArms",
    "nativeArg",
  ])
    add(name, [c, 7n, 1n << 200n]);
  add("structValuedArms", [c, { f: 7n }, { f: 1n << 200n }]);
  for (const [a, b] of [
    [0n, 4294967295n],
    [255n, 0n],
  ])
    add("differingUintWidths", [c, a, b]);
  add("uintArmIntoField", [c, 255n, 1n]);
  for (const n of [0n, (1n << 64n) - 1n])
    add("largeLiteralArithOperand", [c, n]);
}
for (const n of [0n, 5n, 6n, 9n, 10n, 255n]) add("pick", [n]);
const exports = Object.keys(pureCircuits).sort();
if (
  JSON.stringify([...new Set(cases.map((c) => c.export))].sort()) !==
  JSON.stringify(exports)
)
  throw new Error("pure export set differs");
if (new Set(cases.map((c) => c.id)).size !== cases.length)
  throw new Error("duplicate case");
const provenance = {
  sourceSha256: await hash(sourcePath),
  generatedIndexSha256: await hash(contractPath),
  runtimeVersion: pkg.version,
  runtime: {},
};
for (const n of ["index.js", "built-ins.js", "compact-types.js"])
  provenance.runtime[n] = await hash(resolve(runtimeRoot, "dist", n));
process.stdout.write(
  JSON.stringify({ provenance, exports, cases }, null, 2) + "\n",
);
