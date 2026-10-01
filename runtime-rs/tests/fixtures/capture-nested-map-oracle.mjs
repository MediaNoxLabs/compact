// Compile nested_map_oracle.compact with --skip-zk into a fresh target and
// link the generated contract to this branch's compact runtime.
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const { Contract, ledger } = await import(pathToFileURL(contractPath).href);
const contract = new Contract({});
const coinPublicKey = { bytes: new Uint8Array(32) };
const initial = contract.initialState({
  initialPrivateState: null,
  initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
});
const stateHex = () => Buffer.from(initial.currentContractState.serialize()).toString('hex');
const afterInit = {
  stateHex: stateHex(),
  flag: ledger(initial.currentContractState.data).flag,
};
const context = runtime.createCircuitContext(
  runtime.dummyContractAddress(), coinPublicKey,
  initial.currentContractState.data, initial.currentPrivateState,
);
const result = contract.circuits.ping(context);
initial.currentContractState.data = new runtime.ChargedState(
  result.context.currentQueryContext.state.state,
);
const afterPing = {
  stateHex: stateHex(),
  flag: ledger(initial.currentContractState.data).flag,
};
process.stdout.write(JSON.stringify({ afterInit, afterPing }, null, 2) + '\n');
