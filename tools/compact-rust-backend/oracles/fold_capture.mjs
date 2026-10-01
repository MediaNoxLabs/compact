// Capture the exact codegen-rust fold_fixture with ledger-8 TypeScript.
import { createRequire } from 'node:module';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

if (process.argv.length !== 3) throw new Error('usage: node fold_capture.mjs <compiled-contract-dir>');
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
const afterInit = Buffer.from(initial.currentContractState.serialize()).toString('hex');
const counterAfterInit = ledger(initial.currentContractState.data).c.toString();
const context = runtime.createCircuitContext(
  runtime.dummyContractAddress(), coinPublicKey,
  initial.currentContractState.data, initial.currentPrivateState,
);
const afterPing = contract.circuits.ping(context);
const state = new runtime.ContractState();
state.data = new runtime.ChargedState(afterPing.context.currentQueryContext.state.state);
for (const key of initial.currentContractState.operations()) {
  state.setOperation(key, initial.currentContractState.operation(key));
}
state.maintenanceAuthority = initial.currentContractState.maintenanceAuthority;
state.balance = initial.currentContractState.balance;
process.stdout.write(JSON.stringify({
  afterInit, counterAfterInit,
  afterPing: Buffer.from(state.serialize()).toString('hex'),
}, null, 2) + '\n');
