// Capture exact codegen-rust tuple_fixture with ledger-8 TypeScript.
import { createRequire } from 'node:module';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

if (process.argv.length !== 3) throw new Error('usage: node tuple_capture.mjs <compiled-contract-dir>');
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
const normalize = (value) => typeof value === 'bigint' ? value.toString()
  : Array.isArray(value) ? value.map(normalize)
  : value && typeof value === 'object' ? Object.fromEntries(Object.entries(value).map(([k, v]) => [k, normalize(v)]))
  : value;
const results = {
  asVector: normalize(pureCircuits.as_vector(7n)),
  asTuple: normalize(pureCircuits.as_tuple(7n)),
  hetero: normalize(pureCircuits.hetero(9n)),
  oneTuple: normalize(pureCircuits.one_tuple(7n)),
  emptyTuple: normalize(pureCircuits.empty_tuple()),
  tupleCoerce: normalize(pureCircuits.tuple_coerce(9n)),
  tupleVarRef: normalize(pureCircuits.tuple_var_ref(9n)),
  tupleToVector: normalize(pureCircuits.tuple_to_vector(7n)),
  structVectorReturn: normalize(pureCircuits.struct_vector_return(11n)),
  structVectorToTuple: normalize(pureCircuits.struct_vector_to_tuple(11n)),
};
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
  results, afterInit,
  afterPing: Buffer.from(state.serialize()).toString('hex'),
  flagAfterPing: ledger(context.currentQueryContext.state).flag,
}, null, 2) + '\n');
