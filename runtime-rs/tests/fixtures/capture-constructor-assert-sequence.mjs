// Compile constructor_assert_sequence.compact with --skip-zk, link the generated
// contract to this branch's runtime, then pass contract/index.js as argument.
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const { Contract } = await import(pathToFileURL(contractPath).href);
const contract = new Contract({});
const coinPublicKey = { bytes: new Uint8Array(32) };
const constructorContext = () => ({
  initialPrivateState: null,
  initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
});
const initial = contract.initialState(constructorContext(), true);
const context = runtime.createCircuitContext(
  runtime.dummyContractAddress(), coinPublicKey,
  initial.currentContractState.data, initial.currentPrivateState,
);
const read = contract.circuits.read(context);
let failure;
try {
  contract.initialState(constructorContext(), false);
  throw new Error('expected failed assertion');
} catch (error) {
  failure = error.message;
}
process.stdout.write(JSON.stringify({
  stateHex: Buffer.from(initial.currentContractState.serialize()).toString('hex'),
  read: String(read.result),
  failure,
}, null, 2) + '\n');
