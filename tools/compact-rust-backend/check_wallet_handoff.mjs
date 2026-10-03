// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

// Check the exact ledger-v8 binary boundary used by the Midnight wallet SDK.
// The caller installs @midnight-ntwrk/ledger-v8@8.0.2 into a temporary prefix.

import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const [transactionPath, packageRoot] = process.argv.slice(2);
if (!transactionPath || !packageRoot) {
  throw new Error('usage: node check_wallet_handoff.mjs <sealed-transaction.bin> <ledger-v8-package-dir>');
}

const packageDirectory = resolve(packageRoot);
const manifest = JSON.parse(await readFile(resolve(packageDirectory, 'package.json'), 'utf8'));
if (manifest.name !== '@midnight-ntwrk/ledger-v8' || manifest.version !== '8.0.2') {
  throw new Error(`expected @midnight-ntwrk/ledger-v8@8.0.2, got ${manifest.name}@${manifest.version}`);
}

const ledger = await import(pathToFileURL(resolve(packageDirectory, 'midnight_ledger_wasm_fs.js')).href);
const bytes = new Uint8Array(await readFile(transactionPath));
const transaction = ledger.Transaction.deserialize('signature', 'proof', 'binding', bytes);
const intents = transaction.intents;
if (intents?.size !== 1 || intents.get(1)?.actions.length !== 1) {
  throw new Error('expected one proven counter call in segment 1');
}
if (!Buffer.from(transaction.serialize()).equals(Buffer.from(bytes))) {
  throw new Error('ledger-v8 changed sealed transaction bytes after deserialization');
}
console.log(`ledger-v8 8.0.2 decoded and reserialized ${bytes.length} proven transaction bytes`);
