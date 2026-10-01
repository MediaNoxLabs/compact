// Compile nested_collection_query_write.compact with --skip-zk, link generated
// contract to this branch's runtime, then pass contract/index.js as argument.
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const { Contract } = await import(pathToFileURL(contractPath).href);
const contract = new Contract({});
const coinPublicKey = { bytes: new Uint8Array(32) };
const initial = contract.initialState({
  initialPrivateState: null,
  initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
});
let context = runtime.createCircuitContext(
  runtime.dummyContractAddress(), coinPublicKey,
  initial.currentContractState.data, initial.currentPrivateState,
);
function stateHex() {
  initial.currentContractState.data = new runtime.ChargedState(
    context.currentQueryContext.state.state,
  );
  return Buffer.from(initial.currentContractState.serialize()).toString('hex');
}
function call(name, ...args) {
  const result = contract.circuits[name](context, ...args);
  context = result.context;
  return result.result;
}
function flags() {
  return {
    member: call('member_flag'),
    setEmpty: call('set_empty_flag'),
    mapEmpty: call('map_empty_flag'),
  };
}
call('check_member', 42n);
call('check_set_empty');
call('check_map_empty');
const before = { stateHex: stateHex(), flags: flags() };
call('seed', 42n);
call('check_member', 42n);
call('check_set_empty');
call('check_map_empty');
const after = { stateHex: stateHex(), flags: flags() };
process.stdout.write(JSON.stringify({ before, after }, null, 2) + '\n');
