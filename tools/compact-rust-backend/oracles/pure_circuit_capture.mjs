// Capture exact codegen-rust pure_circuit_fixture with ledger-8 TypeScript.
import { createRequire } from 'node:module';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

if (process.argv.length !== 3) throw new Error('usage: node pure_circuit_capture.mjs <compiled-contract-dir>');
const contractIndex = resolve(process.argv[2], 'index.js');
const requireFromContract = createRequire(contractIndex);
const runtime = await import(pathToFileURL(requireFromContract.resolve('@midnight-ntwrk/compact-runtime')));
const { Contract, pureCircuits, ledger } = await import(pathToFileURL(contractIndex));
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
const afterInit = Buffer.from(initial.currentContractState.serialize()).toString('hex');
context = contract.circuits.ping(context).context;
const state = new runtime.ContractState();
state.data = new runtime.ChargedState(context.currentQueryContext.state.state);
for (const key of initial.currentContractState.operations()) {
  state.setOperation(key, initial.currentContractState.operation(key));
}
state.maintenanceAuthority = initial.currentContractState.maintenanceAuthority;
state.balance = initial.currentContractState.balance;
process.stdout.write(JSON.stringify({
  afterInit,
  afterPing: Buffer.from(state.serialize()).toString('hex'),
  flagAfterPing: ledger(context.currentQueryContext.state).flag,
  and: [
    pureCircuits.and_b(false, false), pureCircuits.and_b(false, true),
    pureCircuits.and_b(true, false), pureCircuits.and_b(true, true),
  ],
  whichU32: [pureCircuits.which_u32(false).toString(), pureCircuits.which_u32(true).toString()],
}, null, 2) + '\n');
