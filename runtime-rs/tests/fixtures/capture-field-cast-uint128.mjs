// Compile field_cast_uint128.compact with --skip-zk, link generated contract
// to this branch's runtime, then pass contract/index.js as argument.
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const { Contract, pureCircuits } = await import(pathToFileURL(contractPath).href);
const contract = new Contract({});
const coinPublicKey = { bytes: new Uint8Array(32) };
const initial = contract.initialState({
  initialPrivateState: null,
  initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
});
const input = (1n << 80n) + 7n;
const context = runtime.createCircuitContext(
  runtime.dummyContractAddress(), coinPublicKey,
  initial.currentContractState.data, initial.currentPrivateState,
);
const saved = contract.circuits.save(context, input);
initial.currentContractState.data = new runtime.ChargedState(saved.context.currentQueryContext.state.state);
process.stdout.write(JSON.stringify({
  input: input.toString(),
  pure: pureCircuits.as_field(input).toString(),
  returned: saved.result.toString(),
  afterHex: Buffer.from(initial.currentContractState.serialize()).toString('hex'),
}, null, 2) + '\n');
