// Compile a source fixture with compactc --skip-zk, link its contract
// node_modules/@midnight-ntwrk/compact-runtime to this branch's runtime,
// then run: node capture-collection-transition.mjs <contract/index.js> <set|map|list>.
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const [contractPath, kind] = process.argv.slice(2);
if (!contractPath || !['set', 'map', 'list'].includes(kind)) {
  throw new Error('expected contract/index.js and set, map, or list');
}

const { Contract } = await import(pathToFileURL(contractPath).href);
const contract = new Contract({});
const coinPublicKey = { bytes: new Uint8Array(32) };
const initialZswapLocalState = runtime.emptyZswapLocalState(coinPublicKey);
const initial = contract.initialState({ initialPrivateState: null, initialZswapLocalState });
const context = runtime.createCircuitContext(
  runtime.dummyContractAddress(),
  coinPublicKey,
  initial.currentContractState.data,
  initial.currentPrivateState,
);
const operation = {
  set: () => contract.circuits.add(context, true),
  map: () => contract.circuits.put(context, true, 42n),
  list: () => contract.circuits.prepend(context, 42n),
}[kind];
const result = operation();
initial.currentContractState.data = new runtime.ChargedState(
  result.context.currentQueryContext.state.state,
);
process.stdout.write(JSON.stringify({
  stateHex: Buffer.from(initial.currentContractState.serialize()).toString('hex'),
}, null, 2) + '\n');
