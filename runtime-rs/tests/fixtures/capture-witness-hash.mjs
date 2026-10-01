// Compile witness_hash.compact with --skip-zk, link its generated contract
// to this branch's runtime, then pass contract/index.js as the argument.
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const { Contract } = await import(pathToFileURL(contractPath).href);
const contract = new Contract({
  echo: ({ privateState }, value) => [privateState + 1, value + BigInt(privateState)],
  echo_bytes: ({ privateState }, value) => [privateState + 1, value],
});
const coinPublicKey = { bytes: new Uint8Array(32) };
const opening = Uint8Array.from({ length: 32 }, (_, i) => i + 1);

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
function fieldHex(value) {
  return Array.from({ length: 32 }, (_, index) =>
    Number((value >> BigInt(index * 8)) & 255n).toString(16).padStart(2, '0')).join('');
}
function output(name, kind, ...args) {
  const result = contract.circuits[name](initialContext(), ...args);
  return {
    result: kind === 'field' ? fieldHex(result.result) : Buffer.from(result.result).toString('hex'),
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
  hash: output('hash_echo', 'field', 2n),
  commit: output('commit_echo', 'field', 2n),
  persistentHash: output('persistent_hash_echo', 'bytes', 2n),
  degradeHash: output('degrade_hash_echo', 'field', 2n),
  upgrade: output('upgrade_echo', 'bytes', 2n),
  persistentCommit: output('persistent_commit_echo', 'bytes', 2n, opening),
}, null, 2) + '\n');
