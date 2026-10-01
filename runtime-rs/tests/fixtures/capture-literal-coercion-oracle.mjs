// Compile literal_coercion_oracle.compact with --skip-zk, link generated
// contract to this branch's runtime, then pass contract/index.js as argument.
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const { Contract, pureCircuits } = await import(pathToFileURL(contractPath).href);
const coinPublicKey = { bytes: new Uint8Array(32) };
const initial = new Contract({}).initialState({
  initialPrivateState: null,
  initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
});
process.stdout.write(JSON.stringify({
  initialHex: Buffer.from(initial.currentContractState.serialize()).toString('hex'),
  sameTypeNestedHashHex: Buffer.from(pureCircuits.hashSameTypeNestedArg([5n, 0n])).toString('hex'),
  defaultVectorHashHex: Buffer.from(pureCircuits.hashDefaultVectorArg()).toString('hex'),
  vectorEltWise: pureCircuits.vectorEltWise(7n).map((value) => value.toString()),
  fieldOnly: pureCircuits.callArgFieldOnlyLiteral().toString(),
}, null, 2) + '\n');
