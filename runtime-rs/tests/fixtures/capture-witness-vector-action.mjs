// Compile witness_vector_action.compact with --skip-zk, link generated
// contract to this branch's runtime, then pass contract/index.js as argument.
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const { Contract } = await import(pathToFileURL(contractPath).href);
let calls = 0;
const contract = new Contract({
  sumWitness: ({ privateState }, values) => {
    calls++;
    if (values.length !== 2 || values[0] !== 0n || values[1] !== 1n) {
      throw new Error('unexpected witness vector');
    }
    return [privateState + 1, values[0] + values[1] + BigInt(privateState)];
  },
});
const coinPublicKey = { bytes: new Uint8Array(32) };

function scenario(name) {
  calls = 0;
  const initial = contract.initialState({
    initialPrivateState: 7,
    initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
  });
  const context = runtime.createCircuitContext(
    runtime.dummyContractAddress(), coinPublicKey,
    initial.currentContractState.data, initial.currentPrivateState,
  );
  const result = contract.circuits[name](context);
  initial.currentContractState.data = new runtime.ChargedState(
    result.context.currentQueryContext.state.state,
  );
  return {
    calls,
    privateState: result.context.currentPrivateState,
    stateHex: Buffer.from(initial.currentContractState.serialize()).toString('hex'),
    privateTranscriptOutputs: result.proofData.privateTranscriptOutputs.map(
      ({ value, alignment }) => ({
        valueAtoms: value.map((atom) => Array.from(atom)),
        alignment,
      }),
    ),
  };
}

process.stdout.write(JSON.stringify({
  keepResult: scenario('keepResult'),
  discardResult: scenario('discardResult'),
  reuseResult: scenario('reuseResult'),
}, null, 2) + '\n');
