// Capture ledger-8 TypeScript roots when the path leaf is already a hash.
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

if (process.argv.length !== 4) throw new Error('usage: node merkle_path_root_no_leaf_hash_capture.mjs <compiled-contract-dir> <path-oracle-json>');
const contractIndex = resolve(process.argv[2], 'index.js');
const requireFromContract = createRequire(contractIndex);
await import(pathToFileURL(requireFromContract.resolve('@midnight-ntwrk/compact-runtime')));
const { pureCircuits } = await import(pathToFileURL(contractIndex));
const paths = JSON.parse(readFileSync(process.argv[3], 'utf8'));
function root(key) {
  const data = paths[key];
  const path = {
    leaf: new Uint8Array(32).fill(1),
    path: data.path.map(entry => ({
      sibling: { field: BigInt(entry.sibling) },
      goes_left: entry.goesLeft,
    })),
  };
  return pureCircuits.root_of_hash(path).field.toString();
}
process.stdout.write(JSON.stringify({
  rootWithZeroSiblings: root('pathFor7At0'),
  rootWithNonzeroSibling: root('pathFor9At3'),
}, null, 2) + '\n');
