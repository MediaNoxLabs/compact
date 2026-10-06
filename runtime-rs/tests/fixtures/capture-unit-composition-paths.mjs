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

import { pathToFileURL } from "node:url";
import { readFileSync, realpathSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { createHash } from "node:crypto";
import * as r from "../../../runtime/dist/index.js";
const key = { bytes: new Uint8Array(32) },
  hex = (s) => Buffer.from(s.serialize()).toString("hex");
let queries = [];
const original = r.QueryContext.prototype.query;
r.QueryContext.prototype.query = function (...args) {
  const out = original.call(this, ...args);
  queries.push({ gasCost: out.gasCost });
  return out;
};
const cases = [];
for (const [name, path] of ["flat", "chunked"].map((n, i) => [
  n,
  process.argv[i + 2],
])) {
  if (
    realpathSync(
      resolve(dirname(path), "node_modules/@midnight-ntwrk/compact-runtime"),
    ) !== realpathSync("runtime")
  )
    throw Error("wrong runtime");
  const m = await import(pathToFileURL(path).href),
    c = new m.Contract({ now: ({ privateState }) => [privateState + 1, 42n] });
  for (const take of [false, true]) {
    const initial = c.initialState({
      initialPrivateState: 7,
      initialZswapLocalState: r.emptyZswapLocalState(key),
    });
    const before = hex(initial.currentContractState);
    queries = [];
    const out = c.circuits.maybeClose(
      r.createCircuitContext(
        r.dummyContractAddress(),
        key,
        initial.currentContractState.data,
        7,
      ),
      take,
      0n,
    );
    const own = [...queries];
    initial.currentContractState.data = new r.ChargedState(
      out.context.currentQueryContext.state.state,
    );
    cases.push({
      id: name + "/" + take,
      source: name,
      take,
      result: out.result,
      before,
      after: hex(initial.currentContractState),
      privateAfter: out.context.currentPrivateState,
      queries: own,
      reportedGas: out.gasCost,
      publicTranscript: out.proofData.publicTranscript,
      privateTranscript: out.proofData.privateTranscriptOutputs.map((v) => ({
        valueAtoms: v.value.map((a) => Array.from(a)),
        alignment: v.alignment,
      })),
    });
  }
}
const provenance = [
  "examples/rust_backend/unit_composition_flat.compact",
  "examples/rust_backend/unit_composition_chunked.compact",
  "runtime/src/built-ins.ts",
  "runtime/src/compact-types.ts",
].map((path) => ({
  path,
  sha256: createHash("sha256").update(readFileSync(path)).digest("hex"),
}));
process.stdout.write(
  JSON.stringify(
    { provenance, cases },
    (_, v) =>
      typeof v === "bigint"
        ? String(v)
        : v instanceof Uint8Array
          ? Array.from(v)
          : v,
    2,
  ) + "\n",
);
