// Compile witness_uint_arithmetic with --skip-zk, link its generated contract
// to this branch's runtime, then pass contract/index.js as the argument.
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const { Contract } = await import(pathToFileURL(contractPath).href);
const contract = new Contract({
  echo: ({ privateState }, value) => [privateState + 1, value],
});
const coinPublicKey = { bytes: new Uint8Array(32) };

function initialContext() {
  const initial = contract.initialState({
    initialPrivateState: 7,
    initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
  });
  return runtime.createCircuitContext(
    runtime.dummyContractAddress(),
    coinPublicKey,
    initial.currentContractState.data,
    initial.currentPrivateState,
  );
}

function output(name, value) {
  const result = contract.circuits[name](initialContext(), value);
  return {
    result: result.result.toString(),
    privateState: result.context.currentPrivateState,
    privateTranscriptOutputs: result.proofData.privateTranscriptOutputs.map(
      ({ value, alignment }) => ({
        valueAtoms: value.map((atom) => Array.from(atom)),
        alignment,
      }),
    ),
  };
}

function rejected(name, value) {
  try {
    contract.circuits[name](initialContext(), value);
    return false;
  } catch {
    return true;
  }
}

process.stdout.write(JSON.stringify({
  add: output('add_echo', 2n),
  subtract: output('subtract_echo', 2n),
  multiply: output('multiply_echo', 2n),
  addOverflowRejected: rejected('add_echo', 65535n),
  subtractUnderflowRejected: rejected('subtract_echo', 0n),
  multiplyOverflowRejected: rejected('multiply_echo', 65535n),
}, null, 2) + '\n');
