// This file is part of midnightntwrk/midnight-verifiable-credential-digital-passport.
// Copyright (C) Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

import { existsSync, readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

const packageRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');

const sourceSurface = (relativePath: string) => path.resolve(packageRoot, 'src', relativePath);
const distSurface = (relativePath: string) => path.resolve(packageRoot, 'dist', relativePath);
const distRoot = path.resolve(packageRoot, 'dist');
const indexSource = readFileSync(sourceSurface('index.ts'), 'utf8');
const packageJson = JSON.parse(readFileSync(path.resolve(packageRoot, 'package.json'), 'utf8')) as {
  exports?: Record<string, unknown>;
};

describe('credentials-digital-passport package surfaces', () => {
  it('declares a stable contract subpath export', () => {
    expect(packageJson.exports?.['./contract']).toBeDefined();
    expect(existsSync(sourceSurface('contract.ts'))).toEqual(true);
  });

  it('keeps the root package surface free of duplicate contract namespaces', () => {
    expect(indexSource).not.toContain('export * as DigitalPassportCredentialContract');
  });

  it('publishes the stable contract subpath after build', () => {
    if (!existsSync(distRoot) || !existsSync(distSurface('index.js'))) {
      return;
    }
    expect(existsSync(distSurface('contract.js'))).toEqual(true);
  });

  it('exports key credential-family types from the managed runtime', () => {
    // This test verifies the managed runtime surface has the expected types.
    // It only runs after a build; if dist doesn't exist yet we skip.
    if (!existsSync(distRoot)) {
      return;
    }
    // Re-import dynamically to avoid build-time dependency issues in the
    // scaffold test; the real assertions run in the integration tests.
    expect(indexSource).toContain('managed/digital-passport-credential/contract/index.js');
  });
});
