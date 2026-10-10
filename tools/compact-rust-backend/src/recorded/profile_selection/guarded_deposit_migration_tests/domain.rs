// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn interface_boundary_preserves_single_bytes32_product_and_canonical_coin() {
    for mode in 0..12 {
        let mut c = source();
        let r = root_mut(&mut c);
        match mode {
            0 => r.parameters.clear(),
            1 => r.parameters.push(Parameter {
                name: "extra".into(),
                ty: Type::Boolean,
            }),
            2 => r.parameters.swap(0, 1),
            3 => r.parameters[0].ty = Type::Field,
            4 => r.parameters[1].ty = Type::Bytes { length: 32 },
            5..=7 => {
                let Type::Struct { fields, .. } = &mut r.parameters[1].ty else {
                    unreachable!()
                };
                match mode {
                    5 => fields.clear(),
                    6 => fields.push(fields[0].clone()),
                    7 => fields[0].ty = Type::Bytes { length: 31 },
                    _ => unreachable!(),
                }
            }
            8 => {
                let Type::Struct { name, .. } = &mut r.parameters[2].ty else {
                    unreachable!()
                };
                *name = "LookalikeCoin".into();
            }
            9 => r.result = Type::Field,
            10 => r.return_value = StateReturn::Expression { value: Expr::Unit },
            11 => {
                r.return_value = StateReturn::Effectful {
                    body: ReturnPlan::Value { value: Expr::Unit },
                }
            }
            _ => unreachable!(),
        }
        assert!(
            Context::new(&c).domain(root(&c)).unwrap().is_none(),
            "interface case {mode}"
        );
    }
}

#[test]
fn arbitrary_declared_single_bytes32_product_is_a_body_composition_expansion() {
    for actionful in [false, true] {
        let mut c = source();
        let r = root_mut(&mut c);
        let Type::Struct { name, fields } = &mut r.parameters[1].ty else {
            unreachable!()
        };
        *name = "BeneficiaryEnvelope".into();
        fields[0].name = "account".into();
        r.actions = if actionful {
            vec![StateAction::Expression { value: Expr::Unit }]
        } else {
            vec![]
        };
        let maps = Context::new(&c);
        assert!(
            maps.old(root(&c)).is_none(),
            "explicit expansion, not old inclusion"
        );
        assert!(maps.domain(root(&c)).unwrap().is_some());
        public_owner(&c, "set_topic");
    }
}

#[test]
fn duplicate_source_formals_remain_source_refusals() {
    let mut c = source();
    let r = root_mut(&mut c);
    r.parameters[1].name = r.parameters[0].name.clone();
    let maps = Context::new(&c);
    assert!(matches!(
        effect_plan::check_with_calls(root(&c), &maps.ledger, &maps.witnesses, &maps.pure, &maps.circuits),
        Err(effect_plan::PlanError::Unsupported(gap))
            if gap.path == "parameters[1]" && gap.detail == "duplicate parameter"
    ));
    assert!(maps.domain(root(&c)).unwrap().is_none());
}
