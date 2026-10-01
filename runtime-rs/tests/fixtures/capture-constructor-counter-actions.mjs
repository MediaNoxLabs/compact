// Compile constructor_counter_actions.compact with --skip-zk, link generated
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
}, 1n);
const context = runtime.createCircuitContext(
  runtime.dummyContractAddress(), coinPublicKey,
  initial.currentContractState.data, initial.currentPrivateState,
);
const read = contract.circuits.read_count(context);
const value = contract.circuits.read_value(read.context);
process.stdout.write(JSON.stringify({
  initialHex: Buffer.from(initial.currentContractState.serialize()).toString('hex'),
  count: read.result.toString(),
  value: value.result.toString(),
}, null, 2) + '\n');
