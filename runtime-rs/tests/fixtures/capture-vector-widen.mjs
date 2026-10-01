// Compile vector_widen.compact with --skip-zk, link generated contract to
// this branch's runtime, then pass contract/index.js as argument.
import { pathToFileURL } from 'node:url';
const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const { pureCircuits } = await import(pathToFileURL(contractPath).href);
const values = [7n, 255n];
process.stdout.write(JSON.stringify({
  widened: pureCircuits.widen(values).map((value) => value.toString()),
  elements: pureCircuits.widen_elements(4294967295n).map((value) => value.toString()),
  hashHex: Buffer.from(pureCircuits.hash_widened(values)).toString('hex'),
  nestedHashHex: Buffer.from(pureCircuits.hash_nested(values)).toString('hex'),
}, null, 2) + '\n');
