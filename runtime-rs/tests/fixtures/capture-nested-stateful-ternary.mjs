// Compile nested_stateful_ternary.compact with --skip-zk, link the generated
// contract to this branch's runtime, then pass contract/index.js as argument.
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const { Contract } = await import(pathToFileURL(contractPath).href);
const contract = new Contract({});
const coinPublicKey = { bytes: new Uint8Array(32) };

function run(choice) {
  const initial = contract.initialState({
    initialPrivateState: null,
    initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
  });
  const context = runtime.createCircuitContext(
    runtime.dummyContractAddress(), coinPublicKey,
    initial.currentContractState.data, initial.currentPrivateState,
  );
  const result = contract.circuits.run(context, choice);
  initial.currentContractState.data = new runtime.ChargedState(result.context.currentQueryContext.state.state);
  return Buffer.from(initial.currentContractState.serialize()).toString('hex');
}

process.stdout.write(JSON.stringify({ true: run(true), false: run(false) }, null, 2) + '\n');
