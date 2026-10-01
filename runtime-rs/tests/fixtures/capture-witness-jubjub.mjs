// Compile witness_jubjub.compact with --skip-zk, link generated contract to
// this branch's runtime, then pass contract/index.js.
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const { Contract } = await import(pathToFileURL(contractPath).href);
const contract = new Contract({
  secret_point: ({ privateState }, seed) => [
    privateState + 1,
    runtime.hashToCurve(runtime.CompactTypeField, seed + BigInt(privateState)),
  ],
  secret_scalar: ({ privateState }, seed) => [privateState + 1, seed + BigInt(privateState)],
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
function hex(value) {
  return Array.from({ length: 32 }, (_, index) =>
    Number((value >> BigInt(index * 8)) & 255n).toString(16).padStart(2, '0')).join('');
}
function coords(point) { return { x: hex(point.x), y: hex(point.y) }; }
function output(name, kind) {
  const result = contract.circuits[name](initialContext(), 2n);
  return {
    result: kind === 'field' ? hex(result.result) : coords(result.result),
    privateState: result.context.currentPrivateState,
    privateTranscriptOutputs: result.proofData.privateTranscriptOutputs.map(
      ({ value, alignment }) => ({
        valueAtoms: value.map((atom) => Array.from(atom)),
        alignment,
      }),
    ),
  };
}
process.stdout.write(JSON.stringify({
  combine: output('combine', 'point'),
  scale: output('scale', 'point'),
  pointX: output('point_x', 'field'),
  pointY: output('point_y', 'field'),
  negate: output('negate', 'point'),
  reveal: output('reveal', 'point'),
  hashScalar: output('hash_scalar', 'point'),
  generator: output('generator', 'point'),
}, null, 2) + '\n');
