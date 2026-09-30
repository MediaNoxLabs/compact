// Compile examples/rust_backend/witness_ledger_counter.compact with --skip-zk,
// link the generated contract runtime to this branch's runtime, then run:
// node capture-witness-ledger-counter.mjs <contract/index.js>.
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const { Contract } = await import(pathToFileURL(contractPath).href);
const contract = new Contract({
  read_round: ({ ledger, privateState }) => [privateState + 1, ledger.round],
});
const coinPublicKey = { bytes: new Uint8Array(32) };
const initial = contract.initialState({
  initialPrivateState: 7,
  initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
});
let context = runtime.createCircuitContext(
  runtime.dummyContractAddress(),
  coinPublicKey,
  initial.currentContractState.data,
  initial.currentPrivateState,
);

function output(result) {
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

const before = contract.circuits.private_round(context);
context = contract.circuits.increment_round(before.context).context;
const after = contract.circuits.private_round(context);
process.stdout.write(JSON.stringify({ before: output(before), after: output(after) }, null, 2) + '\n');
