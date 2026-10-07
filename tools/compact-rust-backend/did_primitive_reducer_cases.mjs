// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// Fixed bounded cases over unchanged maintained reducer sources.
export function cases(kind) {
 if(['point-digest','point-guard','alias-set'].includes(kind)) return kind==='point-digest'?[
{id:'update',operation:'update',point:3}, {id:'projection-only',operation:'pure_projection_only',point:3},
{id:'old-point-write',operation:'point_write_only',point:2},{id:'scalar-control',operation:'scalar_control'},
{id:'denied-before-write',operation:'update',point:3,admit:false,error:'unauthorized'},
{id:'inactive-before-witness',operation:'update',point:3,setup:[{operation:'scalar_control'}],error:'inactive'},
]:kind==='point-guard'?[
{id:'different-both',operation:'check',point:3}, {id:'same-controller',operation:'check',point:1,error:'same controller'},
{id:'same-recovery',operation:'check',point:2,error:'same recovery'},
]:[
{id:'insert-unicode',operation:'mutate',value:'référence-東京',mutation:1},
{id:'remove-unicode',operation:'mutate',value:'référence-東京',mutation:2,setup:[{operation:'mutate',value:'référence-東京',mutation:1}]},
{id:'insert-empty',operation:'mutate',value:'',mutation:1},
{id:'duplicate',operation:'mutate',value:'référence-東京',mutation:1,setup:[{operation:'mutate',value:'référence-東京',mutation:1}],error:'already present'},
{id:'missing',operation:'mutate',value:'absent',mutation:2,error:'absent'},
{id:'undefined-before-query',operation:'mutate',value:'absent',mutation:0,error:'undefined mutation'},
];

 const id='référence-東京';
 const service=(m,extra={})=>({operation:'set_service',value:id,typ:'LinkedDomains',endpoint:'https://example.invalid/',mutation:m,...extra});
 const point=(update,extra={})=>({operation:'upsert',value:id,point:3,update,...extra});
 const nested=(update,extra={})=>({operation:'upsert',value:id,kty:0,x:'original',update,...extra});
 const wide=(extra={})=>({operation:'put',value:id,curve:0,label:'label',point:3,choice:2,...extra});
 switch(kind){
 case 'alias-digest':return [
  {id:'unicode-insert',operation:'run',value:id,mutation:1},
  {id:'empty-remove',operation:'run',value:'',mutation:2},
  {id:'undefined-is-only-hashed',operation:'run',value:id,mutation:0},
  {id:'witness-denied',operation:'run',value:id,mutation:1,admit:false,error:'unauthorized'},
 ];
 case 'alias-guard':return ['run','local_control','transitive_control'].flatMap(operation=>[
  {id:operation+'-insert',operation,mutation:1,...(operation==='local_control'?{setup:[{operation:'run',mutation:1}]}:operation==='transitive_control'?{setup:[{operation:'run',mutation:1},{operation:'local_control',mutation:1}]}:{})},
  {id:operation+'-remove',operation,mutation:2},
  {id:operation+'-undefined',operation,mutation:0,error:operation==='run'?'undefined mutation':'local undefined mutation'},
 ]);
 case 'service-map':return [
  {id:'insert-unicode',...service(1)},
  {id:'update-empty',...service(2,{typ:'',endpoint:''}),setup:[service(1)]},
  {id:'remove',operation:'remove_service',value:id,setup:[service(1),service(2,{typ:'',endpoint:''})]},
  {id:'duplicate',...service(1),setup:[service(1)],error:'duplicate service'},
  {id:'update-missing',...service(2),error:'missing service'},
  {id:'remove-missing',operation:'remove_service',value:id,error:'missing service'},
  {id:'undefined-before-query',...service(0),error:'undefined mutation'},
 ];
 case 'point-map':return [
  {id:'insert-unicode',...point(false)},
  {id:'update-point',...point(true,{point:4}),setup:[point(false)]},
  {id:'remove',operation:'remove',value:id,setup:[point(false),point(true,{point:4})]},
  {id:'duplicate-after-nested-member',...point(false),setup:[point(false)],error:'duplicate'},
  {id:'update-missing',...point(true),error:'missing'},
  {id:'remove-missing',operation:'remove',value:id,error:'missing'},
  {id:'insert-empty-key',...point(false,{value:''})},
 ];
 case 'nested-map':return [
  {id:'insert-unicode',...nested(false)},
  {id:'update-empty',...nested(true,{x:''}),setup:[nested(false)]},
  {id:'remove',operation:'remove',value:id,setup:[nested(false),nested(true,{x:''})]},
  {id:'duplicate',...nested(false),setup:[nested(false)],error:'duplicate'},
  {id:'update-missing',...nested(true),error:'missing'},
  {id:'remove-missing',operation:'remove',value:id,error:'missing'},
  {id:'kind-before-query',...nested(false,{kty:1}),error:'kind'},
 ];
 case 'enum-map':return [
  {id:'insert-kind-a',...wide()},
  {id:'replace-kind-b-outer-c',...wide({curve:1}),setup:[wide()]},
  {id:'overwrite-empty',...wide({label:'',point:4}),setup:[wide(),wide({curve:1})]},
  {id:'drop',operation:'drop',value:id,setup:[wide(),wide({curve:1}),wide({label:'',point:4})]},
  {id:'drop-missing',operation:'drop',value:id,error:'missing'},
  {id:'curve-before-write',...wide({curve:2}),error:'curve'},
 ];
 default:throw Error('unreviewed reducer '+kind);
 }
}
export function argumentsFor(kind,s,r){
 switch(kind){
 case 'point-digest':return s.operation==='scalar_control'?[]:[r.ecMulGenerator(BigInt(s.point))];
 case 'point-guard':return [r.ecMulGenerator(BigInt(s.point))];
 case 'alias-set':return [s.value,s.mutation];
 case 'alias-digest':return [s.value,s.mutation];
 case 'alias-guard':return [s.mutation];
 case 'service-map':return s.operation==='remove_service'?[s.value]:[{id:s.value,typ:s.typ,serviceEndpoint:s.endpoint},s.mutation];
 case 'point-map':return s.operation==='remove'?[s.value]:[{id:s.value,publicKey:r.ecMulGenerator(BigInt(s.point))},s.update];
 case 'nested-map':return s.operation==='remove'?[s.value]:[{id:s.value,jwk:{kty:s.kty,x:s.x}},s.update];
 case 'enum-map':return s.operation==='drop'?[s.value]:[{nested:{curve:s.curve,label:s.label,point:r.ecMulGenerator(BigInt(s.point))},id:s.value,choice:s.choice}];
 default:throw Error('unreviewed reducer '+kind);
 }
}
