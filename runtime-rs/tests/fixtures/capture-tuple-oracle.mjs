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

// Compile tuple_oracle.compact with --skip-zk, link generated contract to
// this branch's runtime, then pass contract/index.js as argument.
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const { Contract, pureCircuits } = await import(pathToFileURL(contractPath).href);
const coinPublicKey = { bytes: new Uint8Array(32) };
const initial = new Contract({}).initialState({
  initialPrivateState: null,
  initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
});
const normalize = (value) => {
  if (typeof value === 'bigint') return value.toString();
  if (Array.isArray(value)) return value.map(normalize);
  if (value && typeof value === 'object') {
    return Object.fromEntries(Object.entries(value).map(([key, item]) => [key, normalize(item)]));
  }
  return value;
};
process.stdout.write(JSON.stringify({
  initialHex: Buffer.from(initial.currentContractState.serialize()).toString('hex'),
  asVector: normalize(pureCircuits.as_vector(7n)),
  asTuple: normalize(pureCircuits.as_tuple(7n)),
  hetero: normalize(pureCircuits.hetero(255n)),
  oneTuple: normalize(pureCircuits.one_tuple(7n)),
  emptyTuple: normalize(pureCircuits.empty_tuple()),
  tupleCoerce: normalize(pureCircuits.tuple_coerce(255n)),
  tupleVarRef: normalize(pureCircuits.tuple_var_ref(255n)),
  tupleToVector: normalize(pureCircuits.tuple_to_vector(7n)),
  structVectorReturn: normalize(pureCircuits.struct_vector_return(7n)),
  structVectorToTuple: normalize(pureCircuits.struct_vector_to_tuple(7n)),
  tupleFirst: normalize(pureCircuits.tuple_first(7n, true)),
  tupleSecond: normalize(pureCircuits.tuple_second(7n, true)),
}, null, 2) + '\n');
