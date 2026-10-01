// Compile assert_pure.compact with --skip-zk and pass contract/index.js.
import { pathToFileURL } from 'node:url';

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const { pureCircuits } = await import(pathToFileURL(contractPath).href);
function result(first, second) {
  try {
    return { ok: pureCircuits.checked_value(first, second, 42n).toString() };
  } catch (error) {
    return { error: error.message };
  }
}
process.stdout.write(JSON.stringify({
  pass: result(true, true),
  firstFails: result(false, false),
  secondFails: result(true, false),
}, null, 2) + '\n');
