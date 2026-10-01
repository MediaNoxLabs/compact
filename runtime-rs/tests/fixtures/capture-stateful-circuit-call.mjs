// Compile stateful_circuit_call.compact with --skip-zk, link generated
// contract to this branch's runtime, then pass contract/index.js as argument.
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const { Contract } = await import(pathToFileURL(contractPath).href);
const contract = new Contract({});
const coinPublicKey = { bytes: new Uint8Array(32) };
const initial = contract.initialState({
  initialPrivateState: null,
  initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
});
const context = runtime.createCircuitContext(
  runtime.dummyContractAddress(), coinPublicKey,
  initial.currentContractState.data, initial.currentPrivateState,
);
const called = contract.circuits.bump_twice(context);
initial.currentContractState.data = new runtime.ChargedState(called.context.currentQueryContext.state.state);
process.stdout.write(JSON.stringify({
  afterHex: Buffer.from(initial.currentContractState.serialize()).toString('hex'),
}, null, 2) + '\n');
