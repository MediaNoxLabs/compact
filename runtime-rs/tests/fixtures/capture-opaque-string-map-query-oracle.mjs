// Compile opaque_string_map_query_oracle.compact with --skip-zk into a fresh
// target, link its package to this branch's runtime, then pass contract/index.js.
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const { Contract } = await import(pathToFileURL(contractPath).href);
const contract = new Contract({});
const coinPublicKey = { bytes: new Uint8Array(32) };
const initial = contract.initialState({
  initialPrivateState: 7,
  initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
});
const initialHex = Buffer.from(initial.currentContractState.serialize()).toString('hex');
const context = runtime.createCircuitContext(
  runtime.dummyContractAddress(), coinPublicKey,
  initial.currentContractState.data, initial.currentPrivateState,
);
let beforeError;
try { contract.circuits.ensure(context, 'asset-1'); }
catch (error) { beforeError = error.message; }
const put = contract.circuits.put(context, 'asset-1', 42n);
initial.currentContractState.data = new runtime.ChargedState(
  put.context.currentQueryContext.state.state,
);
const ensured = contract.circuits.ensure(put.context, 'asset-1');
process.stdout.write(JSON.stringify({
  initialHex,
  afterPutHex: Buffer.from(initial.currentContractState.serialize()).toString('hex'),
  beforeError,
  result: String(ensured.result),
}, null, 2) + '\n');
