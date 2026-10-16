// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Pass an installed @midnight-ntwrk/compact-runtime package directory; no network access.
import { pathToFileURL } from 'node:url';
import { readFileSync } from 'node:fs';
const root = process.argv[2];
const pkg = JSON.parse(readFileSync(`${root}/package.json`, 'utf8'));
const r = await import(pathToFileURL(`${root}/dist/index.js`));
const modulus = 52435875175126190479447740508185965837690552500527637822603658699938581184513n;
const bytes = n => Uint8Array.from({length:32}, (_,i) => Number((n >> BigInt(8*i)) & 255n));
const hex = value => Buffer.from(value).toString('hex');
const hash = (type, value) => hex(r.persistentHash(type, value));
const b32 = new r.CompactTypeBytes(32), b3 = new r.CompactTypeBytes(3);
const vector = (n, type) => new r.CompactTypeVector(n, type);
const run = [];
for (const seed of [0n,257n,modulus-1n]) for (const amount of [0,1,65535]) for (const selected of [false,true]) {
 const value = selected ? (seed+7n)%modulus : seed;
 run.push({seed:seed.toString(),encoded:hex(bytes(seed)),amount,selected,
 digest:hash(vector(3,b32),[bytes(BigInt(257+amount)),bytes(seed),bytes(value)]),
 pair:hash(vector(2,b32),[bytes(seed),bytes(seed)])});
}
const products = [[0,0,0],[0,1,255],[97,98,99]].map(value => {const b=Uint8Array.from(value);return {value,
 bytes:hash(b3,b),vector:hash(vector(2,b3),[b,b]),nested:hash(vector(3,b3),[b,b,b]),
 empty:hash(new r.CompactTypeBytes(0),new Uint8Array()),
 emptyVector:hash(vector(0,b3),[]),
 emptyStruct:hash({alignment:()=>[],toValue:()=>[]},{})};});
console.log(JSON.stringify({runtime:pkg.name,version:pkg.version,scope:'Primitive FAB/hash oracle; not full contract TS execution',run,products},null,2));
