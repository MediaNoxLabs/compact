// Compile struct_field_projection.compact with --skip-zk, link generated
// contract to this branch's runtime, then pass contract/index.js as argument.
import { pathToFileURL } from 'node:url';
const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const { pureCircuits } = await import(pathToFileURL(contractPath).href);
const input = { values: [1n, 2n] };
process.stdout.write(JSON.stringify({
  made: pureCircuits.make().values.map((value) => value.toString()),
  projected: pureCircuits.project(input).map((value) => value.toString()),
  hashHex: Buffer.from(pureCircuits.hash_projected(input)).toString('hex'),
}, null, 2) + '\n');
