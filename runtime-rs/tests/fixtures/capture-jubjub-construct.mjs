// Compile jubjub_construct.compact with --skip-zk, link generated contract to
// this branch's runtime, then pass contract/index.js as the argument.
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const { Contract, pureCircuits } = await import(pathToFileURL(contractPath).href);
const point = runtime.hashToCurve(runtime.CompactTypeField, 42n);
const contract = new Contract({ echo: ({ privateState }, value) => [privateState + 1, value] });
const coinPublicKey = { bytes: new Uint8Array(32) };

function hex(value) {
  return Array.from({ length: 32 }, (_, index) =>
    Number((value >> BigInt(index * 8)) & 255n).toString(16).padStart(2, '0')).join('');
}
function coords(value) { return { x: hex(value.x), y: hex(value.y) }; }
function check(name, compiled, native) {
  const actual = coords(compiled);
  if (JSON.stringify(actual) !== JSON.stringify(coords(native))) {
    throw new Error(`${name}: generated TypeScript differs from native`);
  }
  return actual;
}
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
const constructed = pureCircuits.construct_point(point.x, point.y);
const identity = pureCircuits.construct_point(0n, 1n);
const witnessed = contract.circuits.construct_echo(initialContext(), point.x, point.y);
process.stdout.write(JSON.stringify({
  input: coords(point),
  constructed: check('constructed', constructed, runtime.constructJubjubPoint(point.x, point.y)),
  constructedY: hex(pureCircuits.construct_y(point.x, point.y)),
  identity: check('identity', identity, runtime.constructJubjubPoint(0n, 1n)),
  identityY: hex(pureCircuits.construct_y(0n, 1n)),
  witness: {
    result: check('witness', witnessed.result, point),
    privateState: witnessed.context.currentPrivateState,
    privateTranscriptOutputs: witnessed.proofData.privateTranscriptOutputs.map(
      ({ value, alignment }) => ({ valueAtoms: value.map((atom) => Array.from(atom)), alignment }),
    ),
  },
}, null, 2) + '\n');
