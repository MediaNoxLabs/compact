// Capture the ledger-8 TypeScript oracle for asset_registry_oracle.compact.
// Pass the freshly compiled contract directory. Its node_modules must resolve
// @midnight-ntwrk/compact-runtime to this checkout's runtime package.
import { createRequire } from 'node:module';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

if (process.argv.length !== 3) {
  throw new Error('usage: node asset_registry_capture.mjs <compiled-contract-dir>');
}
const contractIndex = resolve(process.argv[2], 'index.js');
const requireFromContract = createRequire(contractIndex);
const runtime = await import(pathToFileURL(requireFromContract.resolve('@midnight-ntwrk/compact-runtime')));
const { Contract, ledger } = await import(pathToFileURL(contractIndex));

const point1 = runtime.hashToCurve(runtime.CompactTypeField, 1n);
const point2 = runtime.hashToCurve(runtime.CompactTypeField, 2n);
const witnesses = {
  localOperatorKey: () => [null, point1],
  localAuditorKey: () => [null, point2],
  currentTimestamp: () => [null, 1_700_000_000n],
};
const contract = new Contract(witnesses);
const coinPublicKey = { bytes: new Uint8Array(32) };
const initial = contract.initialState({
  initialPrivateState: null,
  initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
});
let context = runtime.createCircuitContext(
  runtime.dummyContractAddress(), coinPublicKey,
  initial.currentContractState.data, initial.currentPrivateState,
);
function snapshot(name) {
  initial.currentContractState.data = new runtime.ChargedState(context.currentQueryContext.state.state);
  const view = ledger(initial.currentContractState.data);
  return {
    name,
    stateHex: Buffer.from(initial.currentContractState.serialize()).toString('hex'),
    recordCount: view.recordCount.toString(),
    writeCount: view.writeCount.toString(),
    recordsSize: view.records.size().toString(),
    watchSize: view.watchList.size().toString(),
    retiredSize: view.retiredKeys.size().toString(),
    tagsSize: view.tags.size().toString(),
  };
}
const snapshots = [snapshot('initial')];
const record = {
  code: new Uint8Array(32),
  note: '',
  provenance: { facility: new Uint8Array(32), registeredAt: 0n },
  kind: 1,
  quantity: 0n,
};
const calls = [
  ['tag', 7n],
  ['setRecord', 'asset-1', record, 1],
  ['setWatch', 'asset-1', 1],
  ['setWatch', 'asset-1', 2],
  ['removeRecord', 'asset-1'],
];
for (const [name, ...args] of calls) {
  context = contract.circuits[name](context, ...args).context;
  snapshots.push(snapshot(name));
}
console.log(JSON.stringify(snapshots, null, 2));
