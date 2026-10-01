// Compile pure_call_action.compact with --skip-zk, link generated contract to
// this branch's runtime, then pass contract/index.js as argument.
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
function context() {
  return runtime.createCircuitContext(
    runtime.dummyContractAddress(), coinPublicKey,
    initial.currentContractState.data, initial.currentPrivateState,
  );
}
let zeroError;
try {
  contract.circuits.save(context(), 0n);
} catch (error) {
  zeroError = error.message;
}
if (!zeroError) throw new Error('zero input unexpectedly passed');
const saved = contract.circuits.save(context(), 7n);
initial.currentContractState.data = new runtime.ChargedState(saved.context.currentQueryContext.state.state);
process.stdout.write(JSON.stringify({
  zeroError,
  afterHex: Buffer.from(initial.currentContractState.serialize()).toString('hex'),
}, null, 2) + '\n');
