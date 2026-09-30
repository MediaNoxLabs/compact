// Compile a witness_ledger_set or witness_ledger_map fixture with --skip-zk,
// link its contract runtime to this branch's runtime, then run:
// node capture-witness-ledger-collection.mjs <contract/index.js> <set|map>.
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const [contractPath, kind] = process.argv.slice(2);
if (!contractPath || !['set', 'map'].includes(kind)) {
  throw new Error('expected contract/index.js and set or map');
}
const { Contract } = await import(pathToFileURL(contractPath).href);
const witnesses = kind === 'set'
  ? { contains_true: ({ ledger, privateState }) => {
    const set = ledger.seen;
    if (set.isEmpty() !== (set.size() === 0n)) throw new Error('Set size mismatch');
    return [privateState + 1, set.member(true)];
  } }
  : { has_true: ({ ledger, privateState }) => {
    const map = ledger.table;
    const present = map.member(true);
    if (map.isEmpty() !== (map.size() === 0n)) throw new Error('Map size mismatch');
    if (present && map.lookup(true) !== 42n) throw new Error('Map lookup mismatch');
    return [privateState + 1, present];
  } };
const contract = new Contract(witnesses);
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
    result: result.result,
    privateState: result.context.currentPrivateState,
    privateTranscriptOutputs: result.proofData.privateTranscriptOutputs.map(
      ({ value, alignment }) => ({
        valueAtoms: value.map((atom) => Array.from(atom)),
        alignment,
      }),
    ),
  };
}

const read = (ctx) => kind === 'set'
  ? contract.circuits.private_contains(ctx)
  : contract.circuits.private_has(ctx);
const write = (ctx) => kind === 'set'
  ? contract.circuits.add_true(ctx)
  : contract.circuits.put_true(ctx, 42n);
const before = read(context);
context = write(before.context).context;
const after = read(context);
process.stdout.write(JSON.stringify({ before: output(before), after: output(after) }, null, 2) + '\n');
