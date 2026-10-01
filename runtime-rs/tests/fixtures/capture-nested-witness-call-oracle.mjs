// Compile nested_witness_call_oracle.compact with --skip-zk into a fresh target,
// link the generated contract to this branch's runtime, then pass contract/index.js.
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const { Contract } = await import(pathToFileURL(contractPath).href);
const contract = new Contract({
  secret: ({ privateState }) => [privateState + 1, 7n],
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
const outer = contract.circuits.outer(context);
initial.currentContractState.data = new runtime.ChargedState(
  outer.context.currentQueryContext.state.state,
);
const afterOuterHex = Buffer.from(initial.currentContractState.serialize()).toString('hex');
const outerValue = contract.circuits.outerValue(outer.context);
initial.currentContractState.data = new runtime.ChargedState(
  outerValue.context.currentQueryContext.state.state,
);
process.stdout.write(JSON.stringify({
  initialHex,
  afterOuterHex,
  afterOuterValueHex: Buffer.from(initial.currentContractState.serialize()).toString('hex'),
  privateState: outer.context.currentPrivateState,
  afterOuterValuePrivateState: outerValue.context.currentPrivateState,
  privateTranscriptOutputs: outer.proofData.privateTranscriptOutputs.map(
    ({ value, alignment }) => ({
      valueAtoms: value.map((atom) => Array.from(atom)),
      alignment,
    }),
  ),
  outerValueTranscript: outerValue.proofData.privateTranscriptOutputs.map(
    ({ value, alignment }) => ({
      valueAtoms: value.map((atom) => Array.from(atom)),
      alignment,
    }),
  ),
}, null, 2) + '\n');
