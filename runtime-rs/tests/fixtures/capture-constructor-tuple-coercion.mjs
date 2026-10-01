// Compile constructor_tuple_coercion.compact with --skip-zk, link generated
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
const context = runtime.createCircuitContext(
  runtime.dummyContractAddress(), coinPublicKey,
  initial.currentContractState.data, initial.currentPrivateState,
);
const read = contract.circuits.read_pair(context);
process.stdout.write(JSON.stringify({
  initialHex: Buffer.from(initial.currentContractState.serialize()).toString('hex'),
  constant: pureCircuits.constant().map((value) => value.toString()),
  read: read.result.map((value) => value.toString()),
}, null, 2) + '\n');
