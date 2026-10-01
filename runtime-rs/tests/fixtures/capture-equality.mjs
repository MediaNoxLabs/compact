// Compile equality.compact with --skip-zk, link generated contract to this
// branch's runtime, then pass contract/index.js as the argument.
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
function witness(left, right) {
  const result = contract.circuits.equal_echo(initialContext(), left, right);
  return {
    result: result.result,
    privateState: result.context.currentPrivateState,
    privateTranscriptOutputs: result.proofData.privateTranscriptOutputs.map(
      ({ value, alignment }) => ({ valueAtoms: value.map((atom) => Array.from(atom)), alignment }),
    ),
  };
}
process.stdout.write(JSON.stringify({
  fieldEqual: pureCircuits.equal_field(42n, 42n),
  fieldDifferent: pureCircuits.equal_field(42n, 43n),
  notEqual: pureCircuits.not_equal_field(42n, 43n),
  bytesEqual: pureCircuits.equal_bytes(Uint8Array.of(1, 2, 3, 4), Uint8Array.of(1, 2, 3, 4)),
  bytesDifferent: pureCircuits.equal_bytes(Uint8Array.of(1, 2, 3, 4), Uint8Array.of(1, 2, 3, 5)),
  pairEqual: pureCircuits.equal_pair([3n, 5n], [3n, 5n]),
  pairDifferent: pureCircuits.equal_pair([3n, 5n], [3n, 6n]),
  witnessEqual: witness(2n, 1n),
  witnessDifferent: witness(2n, 2n),
}, null, 2) + '\n');
