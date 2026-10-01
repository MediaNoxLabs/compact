// Capture exact codegen-rust witnesses_fixture with ledger-8 TypeScript.
import { createRequire } from 'node:module';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

if (process.argv.length !== 3) throw new Error('usage: node witnesses_capture.mjs <compiled-contract-dir>');
const contractIndex = resolve(process.argv[2], 'index.js');
const requireFromContract = createRequire(contractIndex);
const runtime = await import(pathToFileURL(requireFromContract.resolve('@midnight-ntwrk/compact-runtime')));
const { Contract, ledger } = await import(pathToFileURL(contractIndex));
const contract = new Contract({
  fetch_field: ({ privateState }) => [privateState + 1, 42n],
  fetch_maybe: ({ privateState }) => [privateState, { is_some: false, value: 0n }],
  echo: ({ privateState }, x) => [privateState, x],
});
const coinPublicKey = { bytes: new Uint8Array(32) };
const initial = contract.initialState({
  initialPrivateState: 7,
  initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
});
const context = runtime.createCircuitContext(
  runtime.dummyContractAddress(), coinPublicKey,
  initial.currentContractState.data, initial.currentPrivateState,
);
const output = contract.circuits.pull(context);
const state = new runtime.ContractState();
state.data = new runtime.ChargedState(output.context.currentQueryContext.state.state);
for (const key of initial.currentContractState.operations()) {
  state.setOperation(key, initial.currentContractState.operation(key));
}
state.maintenanceAuthority = initial.currentContractState.maintenanceAuthority;
state.balance = initial.currentContractState.balance;
process.stdout.write(JSON.stringify({
  afterInit: Buffer.from(initial.currentContractState.serialize()).toString('hex'),
  afterPull: Buffer.from(state.serialize()).toString('hex'),
  valueAfterPull: ledger(output.context.currentQueryContext.state).v.toString(),
  privateStateAfterPull: output.context.currentPrivateState,
  privateTranscriptOutputs: output.proofData.privateTranscriptOutputs.map(
    ({ value, alignment }) => ({
      valueAtoms: value.map(atom => Array.from(atom)),
      alignment,
    }),
  ),
}, null, 2) + '\n');
