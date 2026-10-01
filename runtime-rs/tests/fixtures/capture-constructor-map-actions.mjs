// Compile constructor_map_actions.compact with --skip-zk, link generated
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
const tableSize = contract.circuits.table_size(context);
const historySize = contract.circuits.history_size(tableSize.context);
const tableValue = contract.circuits.get_true(historySize.context);
const historyValue = contract.circuits.get_false_history(tableValue.context);
process.stdout.write(JSON.stringify({
  initialHex: Buffer.from(initial.currentContractState.serialize()).toString('hex'),
  tableSize: tableSize.result.toString(),
  historySize: historySize.result.toString(),
  tableValue: tableValue.result.toString(),
  historyValue: historyValue.result.toString(),
}, null, 2) + '\n');
