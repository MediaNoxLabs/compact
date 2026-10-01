// Compile opaque_string_set_oracle.compact with --skip-zk into a fresh target,
// link its package to this branch's runtime, then pass contract/index.js.
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const { Contract } = await import(pathToFileURL(contractPath).href);
const contract = new Contract({});
const coinPublicKey = { bytes: new Uint8Array(32) };
const initial = contract.initialState({
  initialPrivateState: 7,
  initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
});
const initialHex = Buffer.from(initial.currentContractState.serialize()).toString('hex');
const context = runtime.createCircuitContext(
  runtime.dummyContractAddress(), coinPublicKey,
  initial.currentContractState.data, initial.currentPrivateState,
);
const before = contract.circuits.hasName(context, 'registry-key').result;
const added = contract.circuits.addName(context, 'registry-key');
initial.currentContractState.data = new runtime.ChargedState(
  added.context.currentQueryContext.state.state,
);
process.stdout.write(JSON.stringify({
  initialHex,
  afterAddHex: Buffer.from(initial.currentContractState.serialize()).toString('hex'),
  before,
  after: contract.circuits.hasName(added.context, 'registry-key').result,
  other: contract.circuits.hasName(added.context, 'other').result,
}, null, 2) + '\n');
