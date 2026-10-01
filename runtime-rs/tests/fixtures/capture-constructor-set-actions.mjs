// Compile constructor_set_actions.compact with --skip-zk, link generated
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
const seenTrue = contract.circuits.contains_seen(context, true);
const seenFalse = contract.circuits.contains_seen(seenTrue.context, false);
const historyTrue = contract.circuits.contains_history(seenFalse.context, true);
const historyFalse = contract.circuits.contains_history(historyTrue.context, false);
process.stdout.write(JSON.stringify({
  initialHex: Buffer.from(initial.currentContractState.serialize()).toString('hex'),
  seenTrue: seenTrue.result,
  seenFalse: seenFalse.result,
  historyTrue: historyTrue.result,
  historyFalse: historyFalse.result,
}, null, 2) + '\n');
