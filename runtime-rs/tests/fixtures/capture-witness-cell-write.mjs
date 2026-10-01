// Compile witness_cell_write with --skip-zk, link its generated contract to
// this branch's runtime, then pass contract/index.js as the only argument.
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const { Contract } = await import(pathToFileURL(contractPath).href);
const contract = new Contract({
  secret: ({ ledger, privateState }, seed) => {
    if (seed !== 2n) throw new Error('unexpected witness argument');
    const expectedCell = privateState === 7 ? 0n : 9n;
    if (ledger.cell !== expectedCell) throw new Error('witness saw stale Cell state');
    return [privateState + 1, seed + BigInt(privateState)];
  },
});
const coinPublicKey = { bytes: new Uint8Array(32) };

function initialContext() {
  const initial = contract.initialState({
    initialPrivateState: 7,
    initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
  });
  return runtime.createCircuitContext(
    runtime.dummyContractAddress(),
    coinPublicKey,
    initial.currentContractState.data,
    initial.currentPrivateState,
  );
}

function run(name) {
  const write = contract.circuits[name](initialContext(), 2n);
  const read = contract.circuits.read_cell(write.context);
  return {
    cell: read.result.toString(),
    privateState: write.context.currentPrivateState,
    privateTranscriptOutputs: write.proofData.privateTranscriptOutputs.map(
      ({ value, alignment }) => ({
        valueAtoms: value.map((atom) => Array.from(atom)),
        alignment,
      }),
    ),
  };
}

process.stdout.write(JSON.stringify({
  single: run('write_secret'),
  twice: run('write_twice'),
}, null, 2) + '\n');
