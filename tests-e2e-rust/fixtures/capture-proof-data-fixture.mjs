// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

import { Contract } from '/tmp/proof-data-ts/contract/index.js';
import * as cr from '@midnight-ntwrk/compact-runtime';

const witnesses = {
  first_secret: (ctx) => [ctx.privateState, 5n],
  second_secret: (ctx) => [ctx.privateState, 7n],
};

const contract = new Contract(witnesses);
const emptyCpk = { bytes: new Uint8Array(32) };
const constructorCtx = {
  initialPrivateState: null,
  initialZswapLocalState: cr.emptyZswapLocalState(emptyCpk),
};

const init = contract.initialState(constructorCtx, 11n);
const ctx = cr.createCircuitContext(
  cr.dummyContractAddress(),
  init.currentZswapLocalState,
  init.currentContractState.data,
  init.currentPrivateState,
);
const call = contract.circuits.read_witness_write(ctx, 11n, 30n);

const atomHex = (atom) => Buffer.from(atom.bytes ?? atom).toString('hex');
const alignedSummary = (av) => ({
  valueAtoms: av.value.map(atomHex),
  alignment: av.alignment.map((a) => JSON.stringify(a, (_, v) => typeof v === 'bigint' ? v.toString() : v)),
});
const opTag = (op) => Object.keys(op)[0];
const popeqValues = call.proofData.publicTranscript
  .filter((op) => Object.prototype.hasOwnProperty.call(op, 'popeq'))
  .map((op) => alignedSummary(op.popeq.result));

const fixture = {
  input: alignedSummary(call.proofData.input),
  output: alignedSummary(call.proofData.output),
  privateTranscriptOutputs: call.proofData.privateTranscriptOutputs.map(alignedSummary),
  publicTranscriptTags: call.proofData.publicTranscript.map(opTag),
  popeqValues,
};
process.stdout.write(JSON.stringify(fixture, null, 2) + '\n');
