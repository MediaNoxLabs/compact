// Compile wide_uint_oracle.compact with --skip-zk in a fresh target, link the
// generated contract to this branch's runtime, then pass contract/index.js.
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const { Contract, pureCircuits } = await import(pathToFileURL(contractPath).href);
const maximum = (1n << 248n) - 1n;
const contract = new Contract({
  nextWide: ({ privateState }) => [privateState + 1, maximum],
});
const coinPublicKey = { bytes: new Uint8Array(32) };
const initial = contract.initialState({
  initialPrivateState: 7,
  initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
});
const initialHex = Buffer.from(initial.currentContractState.serialize()).toString('hex');
const context = runtime.createCircuitContext(
  runtime.dummyContractAddress(), coinPublicKey,
  initial.currentContractState.data, initial.currentPrivateState,
);
const write = contract.circuits.writeWide(context);
initial.currentContractState.data = new runtime.ChargedState(
  write.context.currentQueryContext.state.state,
);
const afterWriteHex = Buffer.from(initial.currentContractState.serialize()).toString('hex');
const read = contract.circuits.readWide(write.context);
process.stdout.write(JSON.stringify({
  initialHex,
  afterWriteHex,
  privateState: write.context.currentPrivateState,
  read: String(read.result),
  maxWide: String(pureCircuits.maxWide()),
  privateTranscriptOutputs: write.proofData.privateTranscriptOutputs.map(
    ({ value, alignment }) => ({
      valueAtoms: value.map((atom) => Array.from(atom)),
      alignment,
    }),
  ),
}, null, 2) + '\n');
