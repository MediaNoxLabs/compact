// Compile jubjub_cell.compact with --skip-zk, link its generated contract
// to this branch's runtime, then pass contract/index.js.
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const { Contract } = await import(pathToFileURL(contractPath).href);
const contract = new Contract({});
const coinPublicKey = { bytes: new Uint8Array(32) };
const initial = contract.initialState({
  initialPrivateState: 7,
  initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
});
let context = runtime.createCircuitContext(
  runtime.dummyContractAddress(), coinPublicKey,
  initial.currentContractState.data, initial.currentPrivateState,
);
function hex(value) {
  return Array.from({ length: 32 }, (_, index) =>
    Number((value >> BigInt(index * 8)) & 255n).toString(16).padStart(2, '0')).join('');
}
function coords(point) { return { x: hex(point.x), y: hex(point.y) }; }
const before = coords(contract.circuits.read_point(context).result);
const point = runtime.hashToCurve(runtime.CompactTypeField, 42n);
context = contract.circuits.set_point(context, point).context;
const after = coords(contract.circuits.read_point(context).result);
context = contract.circuits.set_box(context, { point, count: 7n }).context;
const boxed = contract.circuits.read_box(context).result;
process.stdout.write(JSON.stringify({ before, after, boxed: { point: coords(boxed.point), count: boxed.count.toString() } }, null, 2) + '\n');
