// Compile hash_to_curve_vector_constructor.compact with --skip-zk, link
// generated contract to this branch's runtime, then pass index.js as argument.
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
const initialHex = Buffer.from(initial.currentContractState.serialize()).toString('hex');
const context = runtime.createCircuitContext(
  runtime.dummyContractAddress(), coinPublicKey,
  initial.currentContractState.data, initial.currentPrivateState,
);
const pinged = contract.circuits.ping(context);
initial.currentContractState.data = new runtime.ChargedState(pinged.context.currentQueryContext.state.state);
process.stdout.write(JSON.stringify({
  initialHex,
  afterHex: Buffer.from(initial.currentContractState.serialize()).toString('hex'),
}, null, 2) + '\n');
