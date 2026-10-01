// Compile inline_type_scope_oracle.compact with --skip-zk, link generated
// contract to this branch's runtime, then pass contract/index.js as argument.
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const { Contract } = await import(pathToFileURL(contractPath).href);
const contract = new Contract({});
const coinPublicKey = { bytes: new Uint8Array(32) };
const initialState = () => contract.initialState({
  initialPrivateState: null,
  initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
});
const initial = initialState();
const initialHex = Buffer.from(initial.currentContractState.serialize()).toString('hex');
const fieldVector = (length) => new runtime.CompactTypeVector(length, runtime.CompactTypeField);

function scenario(hash, invoke) {
  const state = initialState();
  let context = runtime.createCircuitContext(
    runtime.dummyContractAddress(), coinPublicKey,
    state.currentContractState.data, state.currentPrivateState,
  );
  context = contract.circuits.setHash(context, hash).context;
  const result = invoke(context);
  context = result.context;
  state.currentContractState.data = new runtime.ChargedState(
    context.currentQueryContext.state.state,
  );
  return Buffer.from(state.currentContractState.serialize()).toString('hex');
}

const scalarHash = runtime.persistentHash(runtime.CompactTypeField, 5n);
const scalarStateHex = scenario(scalarHash, (context) =>
  contract.circuits.checkScalarScope(context, [9n, 10n], 5n));
const aggStateHex = scenario(
  runtime.persistentHash(fieldVector(4), [1n, 2n, 3n, 4n]),
  (context) => contract.circuits.checkAggScope(context, [7n, 8n], [1n, 2n, 3n, 4n]),
);
const noCollisionStateHex = scenario(
  runtime.persistentHash(fieldVector(2), [3n, 4n]),
  (context) => contract.circuits.checkNoCollisionScope(context, [3n, 4n]),
);
let scalarMismatch;
try {
  scenario(scalarHash, (context) => contract.circuits.checkScalarScope(context, [9n, 10n], 6n));
  throw new Error('scalar mismatch unexpectedly passed');
} catch (error) {
  scalarMismatch = String(error.message);
}
process.stdout.write(JSON.stringify({
  initialHex,
  scalarStateHex,
  aggStateHex,
  noCollisionStateHex,
  scalarMismatch,
}, null, 2) + '\n');
