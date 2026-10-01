// Capture exact codegen-rust bug11_fixture with ledger-8 TypeScript.
import { createRequire } from 'node:module';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

if (process.argv.length !== 3) throw new Error('usage: node bug11_capture.mjs <compiled-contract-dir>');
const contractIndex = resolve(process.argv[2], 'index.js');
const requireFromContract = createRequire(contractIndex);
const runtime = await import(pathToFileURL(requireFromContract.resolve('@midnight-ntwrk/compact-runtime')));
const { Contract, ledger } = await import(pathToFileURL(contractIndex));
const contract = new Contract({});
const coinPublicKey = { bytes: new Uint8Array(32) };
const initial = contract.initialState({
  initialPrivateState: null,
  initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
});
let context = runtime.createCircuitContext(
  runtime.dummyContractAddress(), coinPublicKey,
  initial.currentContractState.data, initial.currentPrivateState,
);
function snapshot() {
  const state = new runtime.ContractState();
  state.data = new runtime.ChargedState(context.currentQueryContext.state.state);
  for (const key of initial.currentContractState.operations()) {
    state.setOperation(key, initial.currentContractState.operation(key));
  }
  state.maintenanceAuthority = initial.currentContractState.maintenanceAuthority;
  state.balance = initial.currentContractState.balance;
  return Buffer.from(state.serialize()).toString('hex');
}
const afterInit = Buffer.from(initial.currentContractState.serialize()).toString('hex');
context = contract.circuits.set_tiny(context, 99n).context;
const afterTiny99 = snapshot();
context = contract.circuits.set_medium(context, 69999n).context;
const afterMedium69999 = snapshot();
context = contract.circuits.set_wide(context, 4999999999n).context;
const afterWide4999999999 = snapshot();
const view = ledger(context.currentQueryContext.state);
process.stdout.write(JSON.stringify({
  afterInit, afterTiny99, afterMedium69999, afterWide4999999999,
  values: { tiny: view.tiny.toString(), medium: view.medium.toString(), wide: view.wide.toString() },
}, null, 2) + '\n');
