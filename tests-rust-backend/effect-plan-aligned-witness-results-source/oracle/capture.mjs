import assert from "node:assert/strict";
import {
  readFileSync,
  writeFileSync,
  mkdirSync,
  existsSync,
  symlinkSync,
  readdirSync,
  lstatSync,
} from "node:fs";
import { createHash } from "node:crypto";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
const root = new URL(".", import.meta.url).pathname;
const repository = new URL("../../../", import.meta.url).pathname;
const generated = process.argv[2];
const runtime = process.argv[3];
if (!generated || !runtime)
  throw Error(
    "usage: node capture.mjs GENERATED_CONTRACT_DIR OFFICIAL_TS_RUNTIME_DIR",
  );
const links = join(generated, "node_modules/@midnight-ntwrk");
mkdirSync(links, { recursive: true });
if (!existsSync(join(links, "compact-runtime")))
  symlinkSync(runtime, join(links, "compact-runtime"));
const r = await import(pathToFileURL(join(runtime, "dist/index.js"))),
  { Contract, ledger } = await import(
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
const F = r.CompactTypeField,
  S = r.CompactTypeOpaqueString,
  J = r.CompactTypeJubjubPoint,
  B = new r.CompactTypeBytes(32);
const product = (fields) => ({
  alignment: () => fields.flatMap(([type]) => type.alignment()),
  toValue: (v) => fields.flatMap(([type, get]) => type.toValue(get(v))),
});
const entry = product([
    [S, (v) => v.text],
    [J, (v) => v.point],
  ]),
  pair = product([
    [S, (v) => v[0]],
    [J, (v) => v[1]],
  ]);
const nested = product([
    [pair, (v) => v.items[0]],
    [pair, (v) => v.items[1]],
  ]),
  tuple = product(Array.from({ length: 11 }, (_, i) => [B, (v) => v[i]]));
const types = { text: S, entry, nested, tuple11: tuple, unit_text: S };
const point = r.hashToCurve(F, 1n),
  identity = { x: 0n, y: 1n };
const fixed = () => Array.from({ length: 11 }, () => new Uint8Array(32));
const mixed = fixed();
mixed[0][0] = 1;
mixed[10][0] = 255;
const defaults = {
  text: "",
  entry: { text: "", point: identity },
  nested: {
    items: [
      ["", identity],
      ["", point],
    ],
  },
  tuple11: fixed(),
  unit_text: "",
};
const witnessNames = {
  text: "text_value",
  entry: "entry_value",
  nested: "nested_value",
  tuple11: "tuple_value",
  unit_text: "text_value",
};
const queryGas = [],
  oldQuery = r.QueryContext.prototype.query;
r.QueryContext.prototype.query = function (...args) {
  const result = oldQuery.call(this, ...args);
  queryGas.push(result.gasCost);
  return result;
};
const sum = (rows) =>
  Object.fromEntries(
    ["readTime", "computeTime", "bytesWritten", "bytesDeleted"].map((k) => [
      k,
      rows.reduce((n, x) => n + x[k], 0n),
    ]),
  );
const rows = [];
function run({
  id,
  circuit = "text",
  payload = defaults[circuit],
  fallback = defaults[circuit],
  selected = true,
  permit = true,
  fail = null,
}) {
  const events = [],
    name = witnessNames[circuit],
    witnesses = {};
  for (const n of ["text_value", "entry_value", "nested_value", "tuple_value"])
    witnesses[n] = (ctx) => {
      events.push([n, ctx.privateState, ctx.ledger.calls]);
      assert.equal(n, name, `${id}: unexpected callback`);
      if (fail === n) throw Error(`${n} provider refused`);
      return [ctx.privateState + 1, payload];
    };
  witnesses.permit = (ctx) => {
    events.push(["permit", ctx.privateState, ctx.ledger.calls]);
    if (fail === "permit") throw Error("permit provider refused");
    return [ctx.privateState + 1, permit];
  };
  const contract = new Contract(witnesses),
    key = { bytes: new Uint8Array(32) };
  const initialized = contract.initialState({
    initialPrivateState: 5,
    initialZswapLocalState: r.emptyZswapLocalState(key),
  });
  const ctx = r.createCircuitContext(
    r.dummyContractAddress(),
    key,
    initialized.currentContractState.data,
    5,
  );
  const args = circuit === "unit_text" ? [] : [fallback, selected],
    begin = queryGas.length;
  let call, error;
  try {
    call = contract.circuits[circuit](ctx, ...args);
  } catch (e) {
    error = String(e);
  }
  const expectedEvents = [];
  let reason = null;
  if (selected || circuit === "unit_text") {
    expectedEvents.push([name, 5, 1n]);
    if (fail === name) reason = `${name} provider refused`;
  }
  if (!reason) {
    expectedEvents.push(["permit", 5 + expectedEvents.length, 1n]);
    if (fail === "permit") reason = "permit provider refused";
    else if (!permit) reason = "denied";
  }
  assert.deepEqual(events, expectedEvents, id);
  const row = {
    id,
    circuit,
    payload: clean(payload),
    fallback: clean(fallback),
    selected,
    permit,
    fail,
    args: clean(args),
    events: clean(events),
    queryCostSum: clean(sum(queryGas.slice(begin))),
  };
  if (reason) {
    assert.ok(error?.includes(reason), `${id}: ${error}`);
    rows.push({ ...row, status: "refused", reason, error });
    return;
  }
  assert.equal(error, undefined, id);
  const value = selected || circuit === "unit_text" ? payload : fallback,
    type = types[circuit];
  if (circuit === "unit_text") {
    assert.deepEqual(call.result, [], id);
    assert.notEqual(r.transientHash(S, value), 0n, id);
  } else if (circuit === "tuple11")
    assert.deepEqual(call.result, { value: r.persistentHash(type, value) }, id);
  else {
    const p = r.hashToCurve(type, value);
    assert.deepEqual(
      call.result,
      { x: p.x, y: p.y, transient: r.transientHash(type, value) },
      id,
    );
  }
  assert.equal(ledger(call.context.currentQueryContext.state).calls, 1n, id);
  assert.equal(call.context.currentPrivateState, 5 + events.length, id);
  const privateExpected = [];
  if (selected || circuit === "unit_text")
    privateExpected.push({
      value: type.toValue(payload),
      alignment: type.alignment(),
    });
  privateExpected.push({
    value: r.CompactTypeBoolean.toValue(true),
    alignment: r.CompactTypeBoolean.alignment(),
  });
  assert.deepEqual(
    call.proofData.privateTranscriptOutputs,
    privateExpected,
    id,
  );
  let serializedPreimage;
  try {
    const v = r.proofDataIntoSerializedPreimage(
      call.proofData.input,
      call.proofData.output,
      call.proofData.publicTranscript,
      call.proofData.privateTranscriptOutputs,
      circuit,
    );
    serializedPreimage = { status: "success", bytes: v.length };
  } catch (e) {
    serializedPreimage = { status: "refused", error: String(e) };
  }
  rows.push({
    ...row,
    status: "success",
    result: clean(call.result),
    privateState: call.context.currentPrivateState,
    proofData: clean(call.proofData),
    gasCost: clean(call.gasCost),
    serializedPreimage,
  });
}
for (const [id, payload] of [
  ["empty", ""],
  ["ascii", "hello"],
  ["unicode-interior", "λ\0key"],
  ["trailing", "a\0"],
])
  run({ id: `text-${id}`, payload });
run({
  id: "entry-canonical",
  circuit: "entry",
  payload: { text: "λ\0key", point },
});
run({
  id: "entry-trailing",
  circuit: "entry",
  payload: { text: "a\0", point: identity },
});
run({
  id: "nested-canonical",
  circuit: "nested",
  payload: {
    items: [
      ["", identity],
      ["λ\0key", point],
    ],
  },
});
run({
  id: "nested-trailing-later",
  circuit: "nested",
  payload: {
    items: [
      ["first", point],
      ["a\0", identity],
    ],
  },
});
run({ id: "tuple-zero", circuit: "tuple11", payload: fixed() });
run({ id: "tuple-mixed", circuit: "tuple11", payload: mixed });
run({
  id: "lazy-provider-suppressed",
  selected: false,
  payload: "would fail",
  fallback: "fallback",
  fail: "text_value",
});
run({ id: "selected-provider-refusal", fail: "text_value" });
run({ id: "marker-provider-refusal", fail: "permit" });
run({ id: "marker-denied", permit: false });
run({ id: "unit-local-canonical", circuit: "unit_text", payload: "unit" });
assert.equal(new Set(rows.map((x) => x.id)).size, rows.length);
const files = [
  join(
    repository,
    "examples/rust_backend/effect_plan_aligned_witness_results.compact",
  ),
  new URL(import.meta.url),
  join(generated, "compact-rust-ir.json"),
  join(generated, "index.js"),
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
const counts = {
  cases: rows.length,
  executionSuccess: rows.filter((x) => x.status === "success").length,
  executionRefusal: rows.filter((x) => x.status === "refused").length,
  preimageSuccess: rows.filter(
    (x) => x.serializedPreimage?.status === "success",
  ).length,
  preimageRefusal: rows.filter(
    (x) => x.serializedPreimage?.status === "refused",
  ).length,
};
writeFileSync(
  join(root, "typescript.json"),
  JSON.stringify(
    {
      status: "passed",
      scope:
        "Fresh default-v2 source TS execution and independent official primitive descriptor answers. No Rust execution or proof claim.",
      files: Object.fromEntries(
        files.map((p) => [
          p,
          createHash("sha256").update(readFileSync(p)).digest("hex"),
        ]),
      ),
      ...counts,
      rows,
    },
    null,
    2,
  ) + "\n",
);
console.log(JSON.stringify({ status: "passed", ...counts }));
