// Capture ledger-8 TypeScript state for hmt_insert_oracle.compact.
// Pass a compiled contract directory with @midnight-ntwrk/compact-runtime linked.
import { createRequire } from 'node:module';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

if (process.argv.length !== 3) throw new Error('usage: node hmt_insert_capture.mjs <compiled-contract-dir>');
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
const rootAtInit = ledger(initial.currentContractState.data).t.root();
function full() {
  const out = contract.circuits.full(context);
  context = out.context;
  return out.result;
}
function known(root) {
  const out = contract.circuits.known(context, root);
  context = out.context;
  return out.result;
}
function currentRoot() {
  return ledger(new runtime.ChargedState(context.currentQueryContext.state.state)).t.root();
}
const fullAtInit = full();
const knownAtInit = known(rootAtInit);
context = contract.circuits.append(context, 7n).context;
const afterAppend7 = snapshot();
const knownInitialAfterAppend = known(rootAtInit);
context = contract.circuits.place(context, 9n, 3n).context;
const afterPlace9At3 = snapshot();
context = contract.circuits.append(context, 11n).context;
const afterAppend11 = snapshot();
context = contract.circuits.place(context, 13n, 1n).context;
const afterPlace13At1 = snapshot();
const rootBeforeReset = currentRoot();
context = contract.circuits.forget_history(context).context;
const afterForgetHistory = snapshot();
const knownInitialAfterReset = known(rootAtInit);
const knownCurrentAfterReset = known(rootBeforeReset);
const fullBeforeCapacity = full();
context = contract.circuits.append(context, 21n).context;
context = contract.circuits.append(context, 22n).context;
context = contract.circuits.append(context, 23n).context;
const afterCapacity = snapshot();
const fullAtCapacity = full();
process.stdout.write(JSON.stringify({ afterInit, afterAppend7, afterPlace9At3, afterAppend11, afterPlace13At1, afterForgetHistory, fullAtInit, fullBeforeCapacity, afterCapacity, fullAtCapacity, knownAtInit, knownInitialAfterAppend, knownInitialAfterReset, knownCurrentAfterReset }, null, 2) + '\n');
