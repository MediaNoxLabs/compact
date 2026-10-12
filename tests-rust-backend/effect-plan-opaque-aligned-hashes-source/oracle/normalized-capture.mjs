// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Supplement only: original TS source execution outputs remain unchanged.
// Usage: node normalized-capture.mjs ORIGINAL_CAPTURE RUNTIME_PACKAGE OUTPUT
import assert from "node:assert/strict";
import { readFileSync, writeFileSync, readdirSync, lstatSync } from "node:fs";
import { createHash } from "node:crypto";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
const [original, runtime, output] = process.argv.slice(2),
  source = JSON.parse(readFileSync(original));
const r = await import(pathToFileURL(join(runtime, "dist/index.js"))),
  rows = [];
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
for (const row of source.rows) {
  if (row.rustObservedExpectation !== "trailing-zero-refusal") continue;
  const bytes =
    row.type === "string"
      ? [...new TextEncoder().encode(row.input)]
      : [...row.input];
  assert.equal(bytes.at(-1), 0);
  while (bytes.at(-1) === 0) bytes.pop();
  const normalized =
    row.type === "string"
      ? new TextDecoder().decode(Uint8Array.from(bytes))
      : Uint8Array.from(bytes);
  const type =
    row.type === "string"
      ? r.CompactTypeOpaqueString
      : r.CompactTypeOpaqueUint8Array;
  const value =
    row.op === "curve"
      ? r.hashToCurve(type, normalized)
      : row.op === "transient"
        ? r.transientHash(type, normalized)
        : r.transientCommit(type, normalized, 7n);
  const result =
    row.op === "curve"
      ? { x: value.x, y: value.y, value: 0n }
      : { x: 0n, y: 0n, value };
  assert.notDeepEqual(clean(result), row.result, row.id);
  rows.push({
    id: row.id,
    op: row.op,
    type: row.type,
    originalInput: row.input,
    normalizedInput: clean(normalized),
    result: clean(result),
  });
}
assert.equal(rows.length, 9);
const files = [
  original,
  new URL(import.meta.url),
  join(runtime, "package.json"),
  join(runtime, "package-lock.json"),
];
function tree(dir) {
  for (const n of readdirSync(dir).sort()) {
    const p = join(dir, n),
      s = lstatSync(p);
    if (s.isDirectory()) tree(p);
    else if (s.isFile()) files.push(p);
  }
}
tree(join(runtime, "dist"));
tree(join(runtime, "node_modules/@midnight-ntwrk/onchain-runtime-v3"));
const report = {
  status: "passed",
  scope:
    "Independent official TS primitive answers after explicitly stripping trailing zeros, documenting current Rust FAB conversion. Not original-source TS parity; original TS capture is retained unchanged. No Rust primitive is called by this script.",
  caseCount: rows.length,
  files: Object.fromEntries(
    files.map((p) => [
      p,
      createHash("sha256").update(readFileSync(p)).digest("hex"),
    ]),
  ),
  rows,
};
writeFileSync(output, JSON.stringify(report, null, 2) + "\n");
console.log(
  JSON.stringify({
    status: "passed",
    normalizedReferenceCases: rows.length,
    originalCaptureUnchanged: true,
  }),
);
