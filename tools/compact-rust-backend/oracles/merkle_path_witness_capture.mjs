// Capture a Compact MerkleTreePath witness through ledger-8 TypeScript output.
import { createRequire } from 'node:module';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

if (process.argv.length !== 3) throw new Error('usage: node merkle_path_witness_capture.mjs <compiled-contract-dir>');
const contractIndex = resolve(process.argv[2], 'index.js');
const requireFromContract = createRequire(contractIndex);
const runtime = await import(pathToFileURL(requireFromContract.resolve('@midnight-ntwrk/compact-runtime')));
const { Contract } = await import(pathToFileURL(contractIndex));
const contract = new Contract({
  leaf_path: ({ ledger, privateState }) => [privateState, ledger.t.pathForLeaf(0n, 7n)],
});
const coinPublicKey = { bytes: new Uint8Array(32) };
const initial = contract.initialState({
  initialPrivateState: null,
  initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
});
let context = runtime.createCircuitContext(
  runtime.dummyContractAddress(), coinPublicKey,
  initial.currentContractState.data, initial.currentPrivateState,
);
context = contract.circuits.append(context, 7n).context;
const output = contract.circuits.get_path(context);
const path = output.result;
process.stdout.write(JSON.stringify({
  leaf: path.leaf.toString(),
  path: path.path.map(entry => ({
    sibling: entry.sibling.field.toString(),
    goesLeft: entry.goes_left,
  })),
  privateTranscriptOutputs: output.proofData.privateTranscriptOutputs.map(
    ({ value, alignment }) => ({
      valueAtoms: value.map(atom => Array.from(atom)),
      alignment,
    }),
  ),
}, null, 2) + '\n');
