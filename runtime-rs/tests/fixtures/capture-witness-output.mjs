// Compile examples/rust_backend/witness_minimal.compact with compactc --skip-zk,
// link its contract's @midnight-ntwrk/compact-runtime to this branch's runtime,
// then run: node capture-witness-output.mjs <contract/index.js> [minimal|argument].
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const [contractPath, kind = 'minimal'] = process.argv.slice(2);
if (!contractPath) {
  throw new Error('expected contract/index.js');
}
if (!['minimal', 'argument'].includes(kind)) {
  throw new Error('expected minimal or argument');
}

const { Contract } = await import(pathToFileURL(contractPath).href);
const witnesses = kind === 'minimal'
  ? { private_value: ({ privateState }) => [privateState + 1, 42n] }
  : { private_offset: ({ privateState }, value) => {
    if (value !== 2n) throw new Error('unexpected witness argument');
    return [privateState + 1, value + 40n];
  } };
const contract = new Contract(witnesses);
const coinPublicKey = { bytes: new Uint8Array(32) };
const initial = contract.initialState({
  initialPrivateState: 7,
  initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
});
const context = runtime.createCircuitContext(
  runtime.dummyContractAddress(),
  coinPublicKey,
  initial.currentContractState.data,
  initial.currentPrivateState,
);
const result = kind === 'minimal'
  ? contract.circuits.read_private(context)
  : contract.circuits.apply_offset(context, 2n);
process.stdout.write(JSON.stringify({
  result: result.result.toString(),
  privateState: result.context.currentPrivateState,
  privateTranscriptOutputs: result.proofData.privateTranscriptOutputs.map(
    ({ value, alignment }) => ({
      valueAtoms: value.map((atom) => Array.from(atom)),
      alignment,
    }),
  ),
}, null, 2) + '\n');
