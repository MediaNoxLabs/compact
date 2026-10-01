// Compile persistent_hash.compact with --skip-zk, link its generated contract
// to this branch's runtime, then pass contract/index.js as the argument.
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const { pureCircuits } = await import(pathToFileURL(contractPath).href);
const field = 42n;
const pair = [3n, 5n];
const opening = Uint8Array.from({ length: 32 }, (_, i) => i + 1);
const pairType = new runtime.CompactTypeVector(2, runtime.CompactTypeField);
const digest = runtime.persistentHash(runtime.CompactTypeField, field);

function fieldHex(value) {
  return Array.from({ length: 32 }, (_, index) =>
    Number((value >> BigInt(index * 8)) & 255n).toString(16).padStart(2, '0')).join('');
}
function byteHex(value) { return Buffer.from(value).toString('hex'); }
function check(name, compiled, native, encode) {
  const actual = encode(compiled);
  const expected = encode(native);
  if (actual !== expected) throw new Error(`${name}: generated TypeScript differs from native`);
  return actual;
}

process.stdout.write(JSON.stringify({
  hashField: check('hashField', pureCircuits.hash_field(field), digest, byteHex),
  commitField: check('commitField', pureCircuits.commit_field(field, opening), runtime.persistentCommit(runtime.CompactTypeField, field, opening), byteHex),
  hashPair: check('hashPair', pureCircuits.hash_pair(pair), runtime.persistentHash(pairType, pair), byteHex),
  degrade: check('degrade', pureCircuits.degrade_digest(digest), runtime.degradeToTransient(digest), fieldHex),
  upgrade: check('upgrade', pureCircuits.upgrade_field(field), runtime.upgradeFromTransient(field), byteHex),
  degradeHash: check('degradeHash', pureCircuits.degrade_hash(field), runtime.degradeToTransient(digest), fieldHex),
}, null, 2) + '\n');
