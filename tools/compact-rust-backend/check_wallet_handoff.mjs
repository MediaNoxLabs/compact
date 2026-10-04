// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// Check the exact ledger-v8 binary boundary used by the Midnight wallet SDK.
// The caller installs @midnight-ntwrk/ledger-v8@8.0.3 into a temporary prefix.

import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const arguments_ = process.argv.slice(2);
if (arguments_.length !== 2 && arguments_.length !== 3) {
  throw new Error('usage: node check_wallet_handoff.mjs [sealed-deploy.bin] <sealed-call.bin> <ledger-v8-package-dir>');
}
const [deployPath, transactionPath, packageRoot] = arguments_.length === 3
  ? arguments_
  : [undefined, ...arguments_];

const packageDirectory = resolve(packageRoot);
const manifest = JSON.parse(await readFile(resolve(packageDirectory, 'package.json'), 'utf8'));
if (manifest.name !== '@midnight-ntwrk/ledger-v8' || manifest.version !== '8.0.3') {
  throw new Error(`expected @midnight-ntwrk/ledger-v8@8.0.3, got ${manifest.name}@${manifest.version}`);
}

const ledger = await import(pathToFileURL(resolve(packageDirectory, 'midnight_ledger_wasm_fs.js')).href);
const bytes = new Uint8Array(await readFile(transactionPath));
const transaction = ledger.Transaction.deserialize('signature', 'proof', 'binding', bytes);
const intents = transaction.intents;
if (intents?.size !== 1 || intents.get(1)?.actions.length !== 1 ||
    !(intents.get(1).actions[0] instanceof ledger.ContractCall)) {
  throw new Error('expected one proven counter call in segment 1');
}
if (!Buffer.from(transaction.serialize()).equals(Buffer.from(bytes))) {
  throw new Error('ledger-v8 changed sealed transaction bytes after deserialization');
}
if (deployPath) {
  const deployBytes = new Uint8Array(await readFile(deployPath));
  const deployment = ledger.Transaction.deserialize('signature', 'proof', 'binding', deployBytes);
  const deployIntents = deployment.intents;
  if (deployIntents?.size !== 1 || deployIntents.get(1)?.actions.length !== 1 ||
      !(deployIntents.get(1).actions[0] instanceof ledger.ContractDeploy)) {
    throw new Error('expected one sealed contract deployment in segment 1');
  }
  if (!Buffer.from(deployment.serialize()).equals(Buffer.from(deployBytes))) {
    throw new Error('ledger-v8 changed sealed deployment bytes after deserialization');
  }
  const deployAddress = deployIntents.get(1).actions[0].address;
  const callAddress = intents.get(1).actions[0].address;
  if (deployAddress !== callAddress) {
    throw new Error(`deployment and call addresses differ: ${deployAddress} != ${callAddress}`);
  }
  console.log(`ledger-v8 8.0.3 decoded ${deployBytes.length} deployment and ${bytes.length} call bytes at ${deployAddress}`);
} else {
  console.log(`ledger-v8 8.0.3 decoded and reserialized ${bytes.length} proven transaction bytes`);
}
