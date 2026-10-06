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

import { Contract } from '/tmp/proof-data-ts/contract/index.js';
import * as cr from '@midnight-ntwrk/compact-runtime';

const witnesses = {
  first_secret: (ctx) => [ctx.privateState, 5n],
  second_secret: (ctx) => [ctx.privateState, 7n],
};

const fieldAligned = (value) => ({
  value: cr.CompactTypeField.toValue(value),
  alignment: cr.CompactTypeField.alignment(),
});
const emptyAligned = () => ({ value: [], alignment: [] });

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
const call = contract.circuits.read_witness_write(ctx, 16n, 30n);
const nestedCtx = cr.createCircuitContext(
  cr.dummyContractAddress(),
  init.currentZswapLocalState,
  init.currentContractState.data,
  init.currentPrivateState,
);
const nestedCall = contract.circuits.nested_read_witness_write(
  nestedCtx,
  16n,
  30n,
  true,
  37n,
  40n,
);

const atomHex = (atom) => Buffer.from(atom.bytes ?? atom).toString('hex');
const alignedSummary = (av) => ({
  valueAtoms: av.value.map(atomHex),
  alignment: av.alignment.map((a) =>
    JSON.stringify(a, (_, v) => (typeof v === 'bigint' ? v.toString() : v)),
  ),
});
const opTag = (op) => Object.keys(op)[0];
const popeqValues = call.proofData.publicTranscript
  .filter((op) => Object.prototype.hasOwnProperty.call(op, 'popeq'))
  .map((op) => alignedSummary(op.popeq.result));

const fixture = {
  constructor: {
    constructorId: 'constructor',
    input: alignedSummary(fieldAligned(11n)),
    output: alignedSummary(emptyAligned()),
    privateTranscriptOutputs: [alignedSummary(fieldAligned(5n))],
    publicTranscriptTags: ['push', 'push', 'ins'],
  },
  circuit: {
    input: alignedSummary(call.proofData.input),
    output: alignedSummary(call.proofData.output),
    privateTranscriptOutputs: call.proofData.privateTranscriptOutputs.map(alignedSummary),
    publicTranscriptTags: call.proofData.publicTranscript.map(opTag),
    popeqValues,
  },
  nestedCircuit: {
    input: alignedSummary(nestedCall.proofData.input),
    output: alignedSummary(nestedCall.proofData.output),
    privateTranscriptOutputs: nestedCall.proofData.privateTranscriptOutputs.map(alignedSummary),
    publicTranscriptTags: nestedCall.proofData.publicTranscript.map(opTag),
    popeqValues: nestedCall.proofData.publicTranscript
      .filter((op) => Object.prototype.hasOwnProperty.call(op, 'popeq'))
      .map((op) => alignedSummary(op.popeq.result)),
  },
};
process.stdout.write(JSON.stringify(fixture, null, 2) + '\n');
