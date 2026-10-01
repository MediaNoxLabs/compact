// Capture exact codegen-rust if_stmt_fixture with ledger-8 TypeScript.
import { createRequire } from 'node:module';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

if (process.argv.length !== 3) throw new Error('usage: node if_stmt_capture.mjs <compiled-contract-dir>');
const contractIndex = resolve(process.argv[2], 'index.js');
const requireFromContract = createRequire(contractIndex);
const runtime = await import(pathToFileURL(requireFromContract.resolve('@midnight-ntwrk/compact-runtime')));
const { Contract, pureCircuits } = await import(pathToFileURL(contractIndex));
const contract = new Contract({});
const coinPublicKey = { bytes: new Uint8Array(32) };
const initial = contract.initialState({
  initialPrivateState: null,
  initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
});
process.stdout.write(JSON.stringify({
  afterInit: Buffer.from(initial.currentContractState.serialize()).toString('hex'),
  operations: [...initial.currentContractState.operations()],
  classifyTrue: pureCircuits.classify(true),
  classifyFalse: pureCircuits.classify(false),
}, null, 2) + '\n');
