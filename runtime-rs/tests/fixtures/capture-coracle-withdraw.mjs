// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Complete original source; explicit prior state does not claim funded game setup.
import { createRequire } from "node:module";
import { pathToFileURL } from "node:url";
const [path] = process.argv.slice(2);
const r = await import(
  pathToFileURL(createRequire(path).resolve("@midnight-ntwrk/compact-runtime"))
);
const { Contract } = await import(pathToFileURL(path));
const bytes = new r.CompactTypeBytes(32);
const phase = new r.CompactTypeEnum(6, 1),
  vector = new r.CompactTypeVector(2, bytes);
const key = { bytes: new Uint8Array(32) },
  address = r.dummyContractAddress();
const secret = (color) => new Uint8Array(32).fill(color === "red" ? 7 : 8);
const pad = (s) => {
  const b = new Uint8Array(32);
  b.set(new TextEncoder().encode(s));
  return b;
};
const publicKey = (color, keyColor = color) =>
  r.persistentHash(vector, [secret(keyColor), pad(`coracle:${color}`)]);
const cell = (type, value) =>
  r.StateValue.newCell({
    value: type.toValue(value),
    alignment: type.alignment(),
  });
const array = (values) =>
  values.reduce((a, v) => a.arrayPush(v), r.StateValue.newArray());
const hex = (s) => Buffer.from(s.serialize()).toString("hex");
const gas = (v) =>
  Object.fromEntries(Object.entries(v).map(([k, x]) => [k, String(x)]));
let calls = [],
  queries = [],
  active = {};
const c = new Contract({
  local_secret_key: ({ privateState: p }) => {
    calls.push("secret");
    if (active.witnessFailure && calls.length === 2)
      throw Error("selected secret witness failed");
    const wrong = active.impostor || (active.changedSecret && calls.length > 1);
    return [
      { calls: p.calls + 1 },
      wrong ? new Uint8Array(32).fill(9) : secret(active.color),
    ];
  },
  local_board: () => {
    throw Error("unselected local_board called");
  },
  local_set_board: () => {
    throw Error("unselected local_set_board called");
  },
  fresh_nonce: () => {
    throw Error("unselected fresh_nonce called");
  },
});
const query = r.QueryContext.prototype.query;
r.QueryContext.prototype.query = function (...args) {
  const out = query.call(this, ...args);
  queries.push({ ops: args[0], gas: gas(out.gasCost) });
  return out;
};
const u128 = new r.CompactTypeUnsignedInteger((1n << 128n) - 1n, 16),
  u64 = new r.CompactTypeUnsignedInteger((1n << 64n) - 1n, 8);
const coinType = {
  alignment: () =>
    bytes
      .alignment()
      .concat(bytes.alignment(), u128.alignment(), u64.alignment()),
  toValue: (c) =>
    bytes
      .toValue(c.nonce)
      .concat(
        bytes.toValue(c.color),
        u128.toValue(c.value),
        u64.toValue(c.mt_index),
      ),
};
const coin = (nonce, value, index) => ({
  nonce: new Uint8Array(32).fill(nonce),
  color: new Uint8Array(32).fill(2),
  value: BigInt(value),
  mt_index: BigInt(index),
});
let partial;
const withdraw = c._withdraw_0;
c._withdraw_0 = function (context, proof) {
  partial = proof;
  return withdraw.call(this, context, proof);
};
function run(name, color = "red", options = {}) {
  active = { color, ...options };
  calls = [];
  queries = [];
  partial = undefined;
  const init = c.initialState({
    initialPrivateState: { calls: 0 },
    initialZswapLocalState: r.emptyZswapLocalState(key),
  });
  const fields = init.currentContractState.data.state.asArray();
  fields[0] = cell(bytes, publicKey("red"));
  fields[1] = cell(
    bytes,
    publicKey("blue", options.bothPlayers ? "red" : "blue"),
  );
  fields[5] = cell(phase, options.phase ?? (color === "red" ? 5 : 6));
  fields[6] = cell(coinType, coin(3, 42, 0));
  fields[7] = cell(coinType, coin(4, 17, 1));
  fields[8] = cell(coinType, coin(5, 19, 2));
  init.currentContractState.data = new r.ChargedState(array(fields));
  const ownKey = { bytes: new Uint8Array(32).fill(options.keyByte ?? 11) };
  const ctx = r.createCircuitContext(
    address,
    ownKey,
    init.currentContractState.data,
    { calls: 0 },
  );
  ctx.currentZswapLocalState.currentIndex = 3n;
  if (options.missingKey) ctx.currentZswapLocalState.coinPublicKey = undefined;
  if (options.zeroGas)
    ctx.gasLimit = {
      readTime: 0n,
      computeTime: 0n,
      bytesWritten: 0n,
      bytesDeleted: 0n,
    };
  const before = hex(init.currentContractState);
  calls = [];
  queries = [];
  try {
    const out = c.circuits.withdraw(ctx);
    init.currentContractState.data = new r.ChargedState(
      out.context.currentQueryContext.state.state,
    );
    return {
      name,
      color,
      options,
      before,
      after: hex(init.currentContractState),
      result: out.result,
      output: out.proofData.output,
      privateOutputs: out.proofData.privateTranscriptOutputs,
      publicTranscript: out.proofData.publicTranscript,
      privateState: out.context.currentPrivateState,
      witnessCalls: [...calls],
      queries: [...queries],
      effects: out.context.currentQueryContext.effects,
      plan: out.context.currentZswapLocalState,
      gas: gas(out.gasCost),
    };
  } catch (error) {
    return {
      name,
      color,
      options,
      before,
      error: error.message,
      privateOutputPrefix: partial?.privateTranscriptOutputs ?? [],
      witnessCalls: [...calls],
      queries: [...queries],
    };
  }
}
const rows = [
  run("red"),
  run("blue", "blue"),
  run("distinctRecipient", "red", { keyByte: 23 }),
  run("bothPlayersRed", "red", { bothPlayers: true }),
  run("bothPlayersBlueWinner", "red", { bothPlayers: true, phase: 6 }),
];
for (const color of ["red", "blue"])
  for (const option of [
    "impostor",
    "changedSecret",
    "missingKey",
    "witnessFailure",
    "zeroGas",
  ])
    rows.push(run(color + "-" + option, color, { [option]: true }));
for (const color of ["red", "blue"])
  for (const phase of [0, 1, 2, 3, 4, color === "red" ? 6 : 5])
    rows.push(run(color + "-phase" + phase, color, { phase }));
process.stdout.write(
  JSON.stringify(
    rows,
    (_, x) =>
      x instanceof Uint8Array
        ? Array.from(x)
        : typeof x === "bigint"
          ? String(x)
          : x,
    2,
  ) + "\n",
);
