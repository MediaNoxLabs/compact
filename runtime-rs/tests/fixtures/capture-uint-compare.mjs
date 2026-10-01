// Compile uint_compare.compact with --skip-zk, link generated contract to
// this branch's runtime, then pass contract/index.js as the argument.
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const { Contract, pureCircuits } = await import(pathToFileURL(contractPath).href);
const contract = new Contract({
  echo: ({ privateState }, value) => [privateState + 1, value + BigInt(privateState)],
});
const coinPublicKey = { bytes: new Uint8Array(32) };
function initialContext() {
  const initial = contract.initialState({
    initialPrivateState: 7,
    initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
  });
  return runtime.createCircuitContext(
    runtime.dummyContractAddress(), coinPublicKey,
    initial.currentContractState.data, initial.currentPrivateState,
  );
}
function witnessed(left, right) {
  const result = contract.circuits.witnessed_less(initialContext(), left, right);
  return {
    result: result.result,
    privateState: result.context.currentPrivateState,
    privateTranscriptOutputs: result.proofData.privateTranscriptOutputs.map(
      ({ value, alignment }) => ({ valueAtoms: value.map((atom) => Array.from(atom)), alignment }),
    ),
  };
}
process.stdout.write(JSON.stringify({
  less: pureCircuits.less(7n, 9n),
  lessEqualSame: pureCircuits.less_equal(7n, 7n),
  lessEqualDifferent: pureCircuits.less_equal(9n, 7n),
  greater: pureCircuits.greater(9n, 7n),
  greaterEqualSame: pureCircuits.greater_equal(7n, 7n),
  greaterEqualDifferent: pureCircuits.greater_equal(7n, 9n),
  mixedWidth: pureCircuits.mixed_width(8n, 300n),
  witnessTrue: witnessed(2n, 3n),
  witnessFalse: witnessed(3n, 2n),
}, null, 2) + '\n');
