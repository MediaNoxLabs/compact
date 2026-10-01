// Compile chunked_ledger_oracle.compact with --skip-zk in a fresh directory,
// link its contract/node_modules/@midnight-ntwrk/compact-runtime to runtime,
// then pass contract/index.js as the first argument.
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const { Contract, ledger, pureCircuits } = await import(pathToFileURL(contractPath).href);
const contract = new Contract({});
const coinPublicKey = { bytes: new Uint8Array(32) };
const initial = contract.initialState({
  initialPrivateState: null,
  initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
});
const view = ledger(initial.currentContractState.data);
const values = Array.from({ length: 17 }, (_, i) =>
  String(view[`f${String(i).padStart(2, '0')}`]));
process.stdout.write(JSON.stringify({
  stateHex: Buffer.from(initial.currentContractState.serialize()).toString('hex'),
  values,
  active: view.active,
  ping: pureCircuits.ping(true),
}, null, 2) + '\n');
