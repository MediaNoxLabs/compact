// Compile a map oracle with --skip-zk into a fresh directory, link the
// generated contract to this branch's compact runtime, then pass its index.js
// path and ledger field name.
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const [contractPath, field] = process.argv.slice(2);
if (!contractPath || !field) throw new Error('expected contract/index.js and field');
const { Contract, ledger } = await import(pathToFileURL(contractPath).href);
const contract = new Contract({});
const coinPublicKey = { bytes: new Uint8Array(32) };
const initial = contract.initialState({
  initialPrivateState: null,
  initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
});
const stateHex = () => Buffer.from(initial.currentContractState.serialize()).toString('hex');
const values = () => Array.from(ledger(initial.currentContractState.data)[field], String);
const afterInit = { stateHex: stateHex(), values: values() };
const context = runtime.createCircuitContext(
  runtime.dummyContractAddress(), coinPublicKey,
  initial.currentContractState.data, initial.currentPrivateState,
);
const result = contract.circuits.ping(context);
initial.currentContractState.data = new runtime.ChargedState(
  result.context.currentQueryContext.state.state,
);
const afterPing = { stateHex: stateHex(), values: values() };
process.stdout.write(JSON.stringify({ afterInit, afterPing }, null, 2) + '\n');
