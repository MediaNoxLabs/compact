// Compile mixed_width_operand_oracle.compact with --skip-zk, link generated
// contract to this branch's runtime, then pass contract/index.js as argument.
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const { Contract, pureCircuits } = await import(pathToFileURL(contractPath).href);
const coinPublicKey = { bytes: new Uint8Array(32) };
const contract = new Contract({});
const initial = contract.initialState({
  initialPrivateState: null,
  initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
}, 20n, 4n);
const initialHex = Buffer.from(initial.currentContractState.serialize()).toString('hex');
const check = (run) => {
  try {
    return { ok: true, value: run()?.toString() ?? null };
  } catch (error) {
    return { ok: false, message: String(error.message) };
  }
};
const comparisons = Object.fromEntries(
  ['LE', 'LT', 'GT', 'GE', 'EQ', 'NE'].map((operator) => [
    operator,
    [
      check(() => pureCircuits[`assertProduct${operator}`](1073741823n, 4294967292n)),
      check(() => pureCircuits[`assertProduct${operator}`](1073741824n, 4294967295n)),
    ],
  ]),
);
let context = runtime.createCircuitContext(
  runtime.dummyContractAddress(), coinPublicKey,
  initial.currentContractState.data, initial.currentPrivateState,
);
context = contract.circuits.recordPinned(context, 4n, 20n).context;
context = contract.circuits.recordMatching(context, 7n, 7n).context;
initial.currentContractState.data = new runtime.ChargedState(
  context.currentQueryContext.state.state,
);
process.stdout.write(JSON.stringify({
  initialHex,
  stateAfterActionsHex: Buffer.from(initial.currentContractState.serialize()).toString('hex'),
  constructorUnderflow: check(() => contract.initialState({
    initialPrivateState: null,
    initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
  }, 1n, 1n)),
  comparisons,
  sumMixed: check(() => pureCircuits.sumMixed(4294967295n, 255n)),
  productMixed: check(() => pureCircuits.productMixed(4294967295n, 255n)),
  guardedDiff: [
    check(() => pureCircuits.guardedDiff(20n, 4n)),
    check(() => pureCircuits.guardedDiff(1n, 1n)),
  ],
  recordPinnedFailure: check(() => contract.circuits.recordPinned(
    context, 1073741824n, 4294967295n,
  )),
  recordMatchingFailure: check(() => contract.circuits.recordMatching(context, 7n, 8n)),
}, null, 2) + '\n');
