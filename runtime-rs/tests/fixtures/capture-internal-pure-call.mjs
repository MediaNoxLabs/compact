// Compile internal_pure_call.compact with --skip-zk, link generated contract
// to this branch's runtime, then pass contract/index.js as the argument.
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const { Contract, pureCircuits } = await import(pathToFileURL(contractPath).href);
const contract = new Contract({});
const coinPublicKey = { bytes: new Uint8Array(32) };
const initial = contract.initialState({
  initialPrivateState: null,
  initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
});
let context = runtime.createCircuitContext(
  runtime.dummyContractAddress(), coinPublicKey,
  initial.currentContractState.data, initial.currentPrivateState,
);
context = contract.circuits.save(context, 7n).context;
const read = contract.circuits.read_stored(context);
initial.currentContractState.data = new runtime.ChargedState(read.context.currentQueryContext.state.state);
process.stdout.write(JSON.stringify({
  exportedPure: pureCircuits.double_increment(7n).toString(),
  stored: read.result.toString(),
  stateHex: Buffer.from(initial.currentContractState.serialize()).toString('hex'),
}, null, 2) + '\n');
