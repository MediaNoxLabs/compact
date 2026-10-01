// Compile struct_collision_oracle.compact with --skip-zk, link generated
// contract to this branch's runtime, then pass contract/index.js as argument.
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
const initialHex = Buffer.from(initial.currentContractState.serialize()).toString('hex');
let context = runtime.createCircuitContext(
  runtime.dummyContractAddress(), coinPublicKey,
  initial.currentContractState.data, initial.currentPrivateState,
);
context = contract.circuits.runAlpha(context, 5n).context;
context = contract.circuits.runBeta(context, 7n).context;
initial.currentContractState.data = new runtime.ChargedState(
  context.currentQueryContext.state.state,
);
process.stdout.write(JSON.stringify({
  initialHex,
  afterWritesHex: Buffer.from(initial.currentContractState.serialize()).toString('hex'),
  wrapAlpha: pureCircuits.runWrapAlpha(11n).toString(),
  wrapBeta: pureCircuits.runWrapBeta(true),
}, null, 2) + '\n');
