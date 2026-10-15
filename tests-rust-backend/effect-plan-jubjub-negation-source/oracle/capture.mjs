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
const generated = process.argv[2],
  runtime = process.argv[3];
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
const identity = { x: 0n, y: 1n },
  nonidentity = r.hashToCurve(r.CompactTypeField, 1n),
  rows = [],
  queries = [],
  oldQuery = r.QueryContext.prototype.query;
r.QueryContext.prototype.query = function (...args) {
  const result = oldQuery.call(this, ...args);
  queries.push(result.gasCost);
  return result;
};
const sum = (rows) =>
  Object.fromEntries(
    ["readTime", "computeTime", "bytesWritten", "bytesDeleted"].map((k) => [
      k,
      rows.reduce((n, x) => n + x[k], 0n),
    ]),
  );
for (const value of [identity, nonidentity]) {
  assert.deepEqual(
    r.ecNeg(r.ecNeg(value)),
    value,
    "official involution reference",
  );
  assert.deepEqual(
    r.ecAdd(value, r.ecNeg(value)),
    identity,
    "official additive inverse reference",
  );
}
function run({
  id,
  circuit = "coordinates",
  payload = nonidentity,
  fallback = identity,
  selected = true,
  permit = true,
  fail = null,
  expected = r.ecNeg(payload),
}) {
  const events = [],
    contract = new Contract({
      point: (ctx) => {
        events.push(["point", ctx.privateState, ctx.ledger.calls]);
        if (fail === "point") throw Error("point provider refused");
        return [ctx.privateState + 1, payload];
      },
      permit: (ctx) => {
        events.push(["permit", ctx.privateState, ctx.ledger.calls]);
        if (fail === "permit") throw Error("permit provider refused");
        return [ctx.privateState + 1, permit];
      },
    });
  const key = { bytes: new Uint8Array(32) },
    initial = contract.initialState({
      initialPrivateState: 5,
      initialZswapLocalState: r.emptyZswapLocalState(key),
    }),
    ctx = r.createCircuitContext(
      r.dummyContractAddress(),
      key,
      initial.currentContractState.data,
      5,
    ),
    args = circuit === "close" ? [expected] : [fallback, selected],
    start = queries.length;
  let call, error;
  try {
    call = contract.circuits[circuit](ctx, ...args);
  } catch (e) {
    error = String(e);
  }
  const chosen = circuit === "close" || selected ? payload : fallback,
    negated = r.ecNeg(chosen),
    expectedEvents = [];
  let reason = null;
  if (circuit === "close" || selected) {
    expectedEvents.push(["point", 5, 1n]);
    if (fail === "point") reason = "point provider refused";
  }
  if (!reason && circuit === "close") {
    if (negated.x !== expected.x) reason = "wrong x";
    else if (negated.y !== expected.y) reason = "wrong y";
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
    expected: clean(expected),
    negated: clean(negated),
    args: clean(args),
    events: clean(events),
    queryCostSum: clean(sum(queries.slice(start))),
  };
  if (reason) {
    assert.ok(error?.includes(reason), `${id}: ${error}`);
    rows.push({ ...row, status: "refused", reason, error });
    return;
  }
  assert.equal(error, undefined, id);
  assert.deepEqual(call.result, circuit === "close" ? [] : negated, id);
  assert.equal(call.context.currentPrivateState, 5 + events.length, id);
  assert.equal(ledger(call.context.currentQueryContext.state).calls, 1n, id);
  const privateExpected = [];
  if (circuit === "close" || selected)
    privateExpected.push({
      value: r.CompactTypeJubjubPoint.toValue(payload),
      alignment: r.CompactTypeJubjubPoint.alignment(),
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
  const preimage = r.proofDataIntoSerializedPreimage(
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
    privateState: call.context.currentPrivateState,
    proofData: clean(call.proofData),
    gasCost: clean(call.gasCost),
    serializedPreimage: { status: "success", bytes: preimage.length },
  });
}
run({ id: "coordinates-identity", payload: identity });
run({ id: "coordinates-nonidentity" });
run({
  id: "coordinates-lazy-provider-suppressed",
  selected: false,
  fallback: nonidentity,
  fail: "point",
});
run({ id: "coordinates-point-refusal", fail: "point" });
run({ id: "coordinates-permit-refusal", fail: "permit" });
run({ id: "coordinates-denied", permit: false });
run({ id: "close-identity", circuit: "close", payload: identity });
run({ id: "close-nonidentity", circuit: "close" });
run({ id: "close-wrong-x", circuit: "close", expected: identity });
run({ id: "close-point-refusal", circuit: "close", fail: "point" });
assert.equal(new Set(rows.map((v) => v.id)).size, 10);
const files = [
  join(repository, "examples/rust_backend/effect_plan_jubjub_negation.compact"),
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
  executionSuccess: rows.filter((v) => v.status === "success").length,
  executionRefusal: rows.filter((v) => v.status === "refused").length,
  preimageSuccess: rows.filter(
    (v) => v.serializedPreimage?.status === "success",
  ).length,
};
writeFileSync(
  join(root, "typescript.json"),
  JSON.stringify(
    {
      status: "passed",
      scope:
        "Genuine frozen default-v2 source TS execution with independent official negation answers, involution/additive inverse reference laws and private FAB/callback checks; no Rust execution or proof claim.",
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
