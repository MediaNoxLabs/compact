// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

import { pathToFileURL } from 'node:url';
import { writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
if (process.argv.length !== 5) throw new Error('usage: node capture-private-parts.mjs <generated-index.js> <profile> <output.json>');
const { pureCircuits: pure } = await import(pathToFileURL(resolve(process.argv[2])).href);
const b32 = seed => Uint8Array.from({length:32},(_,i)=>(seed+i)&255);
const b64 = seed => Uint8Array.from({length:64},(_,i)=>(seed+i)&255);
const zero = () => new Uint8Array(32);
const privateParts = { claimValues: { firstNameValuePadded:b64(10), lastNameValuePadded:b64(20), dateOfBirthDays:12345n, documentNumberValue:b32(30), issuingStateValue:b32(40) }, openings:{firstNameOpening:b32(100),lastNameOpening:b32(110),dateOfBirthOpening:b32(120),documentNumberOpening:b32(130),issuingStateOpening:b32(140)} };
const claimCommitments = {
 firstNameCommitment:pure.firstNameCommitment(privateParts.claimValues.firstNameValuePadded,privateParts.openings.firstNameOpening),
 lastNameCommitment:pure.lastNameCommitment(privateParts.claimValues.lastNameValuePadded,privateParts.openings.lastNameOpening),
 dateOfBirthCommitment:pure.dateOfBirthCommitment(privateParts.claimValues.dateOfBirthDays,privateParts.openings.dateOfBirthOpening),
 documentNumberCommitment:pure.documentNumberCommitment(privateParts.claimValues.documentNumberValue,privateParts.openings.documentNumberOpening),
 issuingStateCommitment:pure.issuingStateCommitment(privateParts.claimValues.issuingStateValue,privateParts.openings.issuingStateOpening)
};
const claimRoot=pure.digitalPassportClaimRoot(claimCommitments);
const run=(name,com=claimCommitments,root=claimRoot,parts=privateParts)=>{try{pure.assertValidDigitalPassportCredentialPrivateParts(com,root,parts);return{name,outcome:'ok'}}catch(error){return{name,outcome:'error',message:String(error.message)}}};
const mut=(field,value)=>({...privateParts,claimValues:{...privateParts.claimValues,[field]:value}});
const open=(field,value)=>({...privateParts,openings:{...privateParts.openings,[field]:value}});
const absent={...claimCommitments,documentNumberCommitment:pure.documentNumberNullCommitment()};
const absentRoot=pure.digitalPassportClaimRoot(absent);
const absentParts=mut('documentNumberValue',zero());absentParts.openings={...privateParts.openings,documentNumberOpening:zero()};
const rows=[
 run('private_parts_valid'),
 run('private_parts_wrong_first_opening',claimCommitments,claimRoot,open('firstNameOpening',b32(101))),
 run('private_parts_wrong_last_value',claimCommitments,claimRoot,mut('lastNameValuePadded',b64(21))),
 run('private_parts_wrong_dob_opening',claimCommitments,claimRoot,open('dateOfBirthOpening',b32(121))),
 run('private_parts_wrong_doc_value',claimCommitments,claimRoot,mut('documentNumberValue',b32(31))),
 run('private_parts_wrong_doc_opening',claimCommitments,claimRoot,open('documentNumberOpening',b32(131))),
 run('private_parts_wrong_issuing_state',claimCommitments,claimRoot,mut('issuingStateValue',b32(41))),
 run('private_parts_wrong_root',claimCommitments,b32(99)),
 run('private_parts_absent_valid',absent,absentRoot,absentParts),
 run('private_parts_absent_nonzero_value',absent,absentRoot,{...absentParts,claimValues:{...absentParts.claimValues,documentNumberValue:b32(30)}}),
 run('private_parts_absent_nonzero_opening',absent,absentRoot,{...absentParts,openings:{...absentParts.openings,documentNumberOpening:b32(130)}}),
];
writeFileSync(process.argv[4], JSON.stringify({ profile: process.argv[3], rows }, null, 2) + '\n');
console.log(`${rows.length} private-parts cases: ${rows.filter(row => row.outcome === 'ok').length} ok`);
