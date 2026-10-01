// Capture exact codegen-rust assert_parity_fixture with ledger-8 TypeScript.
import { createRequire } from 'node:module';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

if (process.argv.length !== 3) throw new Error('usage: node assert_parity_capture.mjs <compiled-contract-dir>');
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
function outcome(call) {
  try { return { ok: call() }; }
  catch (error) { return { error: error.message, compactError: error instanceof runtime.CompactError }; }
}
const afterInit = Buffer.from(initial.currentContractState.serialize()).toString('hex');
const pureTrue = outcome(() => pureCircuits.require_true(true));
const pureFalse = outcome(() => pureCircuits.require_true(false));
context = contract.circuits.trigger_ok(context).context;
const afterTriggerOk = snapshot();
const flagAfterTriggerOk = ledger(context.currentQueryContext.state).flag;
const triggerFail = outcome(() => contract.circuits.trigger_fail(context));
const afterTriggerFail = snapshot();
process.stdout.write(JSON.stringify({
  afterInit, pureTrue, pureFalse, afterTriggerOk, flagAfterTriggerOk,
  triggerFail, afterTriggerFail,
}, null, 2) + '\n');
