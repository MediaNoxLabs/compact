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

//! Frozen complete ordinary generic renderer and helper planner from signed617.
//! Two explicit adapters exclude Effectful and bypass modern checked selection.
//! Test-only support authority; published historical diagnostics are measured separately.
use super::*;

mod profile_selection {
    use super::*;
    pub(super) struct ProfileSelection {
        pub(super) organizer_steps: Option<Vec<syn::Stmt>>,
        pub(super) typed_result: Option<syn::Expr>,
        pub(super) shielded_send_rejection: Option<RecordingGap>,
        pub(super) unit_composition_rejection: Option<effect_plan::LoweringRejection>,
    }
}

pub(crate) fn render_recorded_circuit(
    circuit: &StatefulCircuit,
    ledger_fields: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    pure_circuits: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
    shared_callees: &HashSet<String>,
) -> Result<RecordingOutcome<syn::Item>, RenderError> {
    #[cfg(test)]
    let trace = admission_probe::trace::Scope::begin(circuit, false);
    let result = render_recorded_item(
        circuit,
        ledger_fields,
        witnesses,
        pure_circuits,
        circuits,
        shared_callees,
        false,
    );
    #[cfg(test)]
    trace.finish(&result);
    result
}

pub(crate) fn render_recorded_helper(
    circuit: &StatefulCircuit,
    ledger_fields: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    pure_circuits: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
    shared_callees: &HashSet<String>,
) -> Result<RecordingOutcome<syn::Item>, RenderError> {
    #[cfg(test)]
    let trace = admission_probe::trace::Scope::begin(circuit, true);
    let result = render_recorded_item(
        circuit,
        ledger_fields,
        witnesses,
        pure_circuits,
        circuits,
        shared_callees,
        true,
    );
    #[cfg(test)]
    trace.finish(&result);
    result
}

fn collect_field_callees(value: &Expr, names: &mut HashSet<String>) {
    match value {
        Expr::Call { name, .. } => {
            names.insert(name.clone());
        }
        Expr::Add { left, right } => {
            collect_field_callees(left, names);
            collect_field_callees(right, names);
        }
        Expr::Coerce { value, ty } if *ty == Type::Field => {
            collect_field_callees(value, names);
        }
        _ => {}
    }
}

fn collect_shared_callees(action: &StateAction, names: &mut HashSet<String>) {
    match action {
        StateAction::CircuitCall { name, .. } => {
            names.insert(name.clone());
        }
        StateAction::Sequence { actions } => {
            for action in actions {
                collect_shared_callees(action, names);
            }
        }
        StateAction::If {
            then, otherwise, ..
        } => {
            collect_shared_callees(then, names);
            collect_shared_callees(otherwise, names);
        }
        StateAction::Let { bindings, action } => {
            for binding in bindings {
                if binding.ty == Type::Field {
                    collect_field_callees(&binding.value, names);
                }
            }
            collect_shared_callees(action, names);
        }
        StateAction::CellWrite { value, .. } => collect_field_callees(value, names),
        _ => {}
    }
}

/// Find recordable Unit and Field-value callees before emitting public entry points. Rendering each
/// candidate without sharing also checks transitive support and rejects recursion.
pub(crate) fn plan_recorded_helpers(
    ordered_circuits: &[StatefulCircuit],
    ledger_fields: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    pure_circuits: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
) -> Result<(HashSet<String>, Vec<syn::Item>), RenderError> {
    #[cfg(test)]
    let _phase = admission_probe::trace::HelperPlanning::begin();
    let mut roots_with_calls = HashSet::new();
    for circuit in ordered_circuits.iter().filter(|circuit| !circuit.internal) {
        let mut calls = HashSet::new();
        for action in &circuit.actions {
            collect_shared_callees(action, &mut calls);
        }
        if !calls.is_empty() {
            roots_with_calls.insert(circuit.name.as_str());
        }
    }
    if roots_with_calls.is_empty() {
        return Ok((HashSet::new(), Vec::new()));
    }

    // A closed root can require a shared Unit continuation before the root is
    // itself recordable. Validate prospective helpers first so admission does
    // not depend on an unrelated exported circuit using the same helper.
    let mut prospective = HashSet::new();
    for circuit in ordered_circuits
        .iter()
        .filter(|circuit| roots_with_calls.contains(circuit.name.as_str()))
    {
        for action in &circuit.actions {
            collect_shared_callees(action, &mut prospective);
        }
    }
    loop {
        let previous = prospective.len();
        let current = prospective.clone();
        for circuit in ordered_circuits
            .iter()
            .filter(|circuit| current.contains(&circuit.name))
        {
            for action in &circuit.actions {
                collect_shared_callees(action, &mut prospective);
            }
        }
        if prospective.len() == previous {
            break;
        }
    }
    let mut provisional_names = HashSet::new();
    for circuit in ordered_circuits {
        let unit_body = circuit.result == Type::Unit && circuit.return_value == StateReturn::Unit;
        let direct_field_body = circuit.result == Type::Field
            && circuit.actions.is_empty()
            && matches!(circuit.return_value, StateReturn::Expression { .. });
        if !prospective.contains(&circuit.name) || !(unit_body || direct_field_body) {
            continue;
        }
        if crate::located(circuit.source.as_ref(), || {
            render_recorded_helper(
                circuit,
                ledger_fields,
                witnesses,
                pure_circuits,
                circuits,
                &HashSet::new(),
            )
        })?
        .is_supported()
        {
            provisional_names.insert(circuit.name.clone());
        }
    }

    let mut candidates = HashSet::new();
    for circuit in ordered_circuits
        .iter()
        .filter(|circuit| roots_with_calls.contains(circuit.name.as_str()))
    {
        let recorded = crate::located(circuit.source.as_ref(), || {
            render_recorded_circuit(
                circuit,
                ledger_fields,
                witnesses,
                pure_circuits,
                circuits,
                &provisional_names,
            )
        })?;
        if !recorded.is_supported() {
            continue;
        }
        for action in &circuit.actions {
            collect_shared_callees(action, &mut candidates);
        }
    }
    loop {
        let previous = candidates.len();
        let current = candidates.clone();
        for circuit in ordered_circuits
            .iter()
            .filter(|circuit| current.contains(&circuit.name))
        {
            for action in &circuit.actions {
                collect_shared_callees(action, &mut candidates);
            }
        }
        if candidates.len() == previous {
            break;
        }
    }
    let mut names = HashSet::new();
    for circuit in ordered_circuits {
        let unit_body = circuit.result == Type::Unit && circuit.return_value == StateReturn::Unit;
        let direct_field_body = circuit.result == Type::Field
            && circuit.actions.is_empty()
            && matches!(circuit.return_value, StateReturn::Expression { .. });
        if !candidates.contains(&circuit.name) || !(unit_body || direct_field_body) {
            continue;
        }
        if crate::located(circuit.source.as_ref(), || {
            render_recorded_helper(
                circuit,
                ledger_fields,
                witnesses,
                pure_circuits,
                circuits,
                &HashSet::new(),
            )
        })?
        .is_supported()
        {
            names.insert(circuit.name.clone());
        }
    }
    let mut items = Vec::new();
    for circuit in ordered_circuits {
        if !names.contains(&circuit.name) {
            continue;
        }
        let item = crate::located(circuit.source.as_ref(), || {
            render_recorded_helper(
                circuit,
                ledger_fields,
                witnesses,
                pure_circuits,
                circuits,
                &names,
            )
        })?;
        if let RecordingOutcome::Supported(item) = item {
            items.push(item);
        }
    }
    Ok((names, items))
}

fn render_recorded_item(
    circuit: &StatefulCircuit,
    ledger_fields: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    pure_circuits: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
    shared_callees: &HashSet<String>,
    helper: bool,
) -> Result<RecordingOutcome<syn::Item>, RenderError> {
    // Explicit oracle scope: Effectful admission was retired separately.
    // This refusal is an adapter exclusion, never an ordinary old-domain result.
    if matches!(circuit.return_value, StateReturn::Effectful { .. }) {
        return Ok(RecordingOutcome::Unsupported(RecordingGap::returned(
            &circuit.return_value,
        )));
    }
    if circuit.internal && !helper {
        return Ok(RecordingOutcome::Unsupported(RecordingGap::no_effect()));
    }

    let name = ident(&circuit.name)?;
    let mut parameters = HashMap::new();
    let mut args = Vec::<syn::FnArg>::new();
    for (index, parameter) in circuit.parameters.iter().enumerate() {
        ident(&parameter.name)?;
        let rust_name = syn::Ident::new(&format!("__compact_param_{index}"), Span::call_site());
        if parameters
            .insert(parameter.name.as_str(), (&parameter.ty, rust_name.clone()))
            .is_some()
        {
            return Err(RenderError::DuplicateParameter(parameter.name.clone()));
        }
        let arg_ty = rust_type(&parameter.ty)?;
        args.push(syn::parse_quote!(#rust_name: #arg_ty));
    }

    // Independent old generic authority: do not invoke the checked lowerer.
    // Refusal-only modern Unit/Shielded projections are measured separately.
    let profile_selection::ProfileSelection {
        organizer_steps,
        typed_result,
        shielded_send_rejection,
        unit_composition_rejection,
    } = profile_selection::ProfileSelection {
        organizer_steps: None,
        typed_result: None,
        shielded_send_rejection: None,
        unit_composition_rejection: None,
    };
    let organizer_gate = organizer_steps.is_some();
    let mut steps = organizer_steps.unwrap_or_default();
    let mut next_temp = 0;
    let mut visiting = HashSet::from([circuit.name.clone()]);
    // A root Let encloses both its ordered actions and its final return.
    // Capture this exact Field Cell read once, before the nested actions,
    // so the returned value cannot accidentally observe the post-write state.
    let root_field_return = if !organizer_gate {
        match (circuit.actions.as_slice(), &circuit.return_value) {
            (
                [StateAction::Let { bindings, action }],
                StateReturn::Expression {
                    value: Expr::Parameter { name },
                },
            ) if circuit.result == Type::Field => match bindings.as_slice() {
                [binding]
                    if binding.name == *name
                        && binding.ty == Type::Field
                        && matches!(&binding.value, Expr::CellRead { .. }) =>
                {
                    if let Expr::CellRead { field, index } = &binding.value {
                        let declaration = ledger_fields
                            .get(field.as_str())
                            .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                        if declaration.declaration != (LedgerFieldKind::Cell { ty: Type::Field })
                            || declaration.index != *index
                        {
                            return Ok(RecordingOutcome::Unsupported(RecordingGap::returned(
                                &circuit.return_value,
                            )));
                        }
                        Some((binding.name.as_str(), field.as_str(), action.as_ref()))
                    } else {
                        None
                    }
                }
                _ => None,
            },
            _ => None,
        }
    } else {
        None
    };
    let mut recorded_root_return: Option<(&str, syn::Expr)> = None;
    if !organizer_gate {
        if let Some((name, field, action)) = root_field_return {
            #[cfg(test)]
            admission_probe::trace::action("guard.root-let-snapshot");
            let slot = ident(field)?;
            let saved = syn::Ident::new("__compact_recorded_root_return", Span::call_site());
            steps.push(syn::parse_quote! {
                let (frame, #saved): (_, runtime::Field) =
                    crate::ledger_slots::#slot.record_read(frame)?;
            });
            let locals = HashMap::from([(name.to_owned(), syn::parse_quote!(#saved))]);
            if let RecordingOutcome::Unsupported(gap) = append_steps(
                action,
                "actions[0].action",
                &locals,
                &parameters,
                ledger_fields,
                witnesses,
                pure_circuits,
                circuits,
                shared_callees,
                &mut steps,
                &mut next_temp,
                &mut visiting,
                &mut LegacyActionPrecision::Concrete,
            )? {
                return Ok(RecordingOutcome::Unsupported(gap));
            }
            recorded_root_return = Some((name, syn::parse_quote!(#saved)));
        } else {
            #[cfg(test)]
            if !circuit.actions.is_empty() {
                admission_probe::trace::action("legacy.action-dispatch");
            }
            for (index, action) in circuit.actions.iter().enumerate() {
                let mut legacy_precision = LegacyActionPrecision::Concrete;
                if let RecordingOutcome::Unsupported(gap) = append_steps(
                    action,
                    &format!("actions[{index}]"),
                    &HashMap::new(),
                    &parameters,
                    ledger_fields,
                    witnesses,
                    pure_circuits,
                    circuits,
                    shared_callees,
                    &mut steps,
                    &mut next_temp,
                    &mut visiting,
                    &mut legacy_precision,
                )? {
                    let gap = unit_composition_rejection.as_ref().map_or_else(
                        || gap.clone(),
                        |rejection| rejection.refine_action(index, legacy_precision, gap.clone()),
                    );
                    return Ok(RecordingOutcome::Unsupported(gap));
                }
            }
        }
    }
    // Checked composition has inspected the entire value tree. With no
    // accepted typed or legacy profile (and no actions), the generic return
    // arms cannot record a ShieldedSendResult. Keep the first concrete nested
    // composition failure instead of collapsing it to UnsupportedReturn for an outer If
    // or Call. Action diagnostics above still take precedence.
    if !organizer_gate
        && circuit.actions.is_empty()
        && steps.is_empty()
        && let Some(gap) = shielded_send_rejection.as_ref()
        && gap.code == RecordingGapCode::UnsupportedExpression
    {
        return Ok(RecordingOutcome::Unsupported(gap.clone()));
    }
    let result_ty = rust_type(&circuit.result)?;
    #[cfg(test)]
    if typed_result.is_none() {
        admission_probe::trace::returned();
    }
    let (return_steps, result): (Vec<syn::Stmt>, syn::Expr) = match &circuit.return_value {
        _ if typed_result.is_some() => (
            Vec::new(),
            typed_result.expect("typed plan result was checked"),
        ),
        StateReturn::Expression {
            value: Expr::Let { bindings, body },
        } if circuit.result == Type::Unit
            && circuit.actions.is_empty()
            && matches!(body.as_ref(), Expr::Unit)
            && bindings.iter().all(|binding| binding.ty == Type::Field)
            && bindings
                .iter()
                .any(|binding| contains_cell_read(&binding.value)) =>
        {
            // An unused value may still perform a public ledger read. Evaluate
            // each binding in lexical order before discarding the unit result.
            let mut return_steps = Vec::new();
            let mut locals = HashMap::new();
            for (index, binding) in bindings.iter().enumerate() {
                let Some(value) = field_expression(
                    &binding.value,
                    &locals,
                    &parameters,
                    ledger_fields,
                    witnesses,
                    circuits,
                    shared_callees,
                    &mut return_steps,
                    &mut next_temp,
                    &mut visiting,
                )?
                else {
                    return Ok(RecordingOutcome::Unsupported(RecordingGap::expression(
                        &binding.value,
                        format!("return_value.value.bindings[{index}].value"),
                    )));
                };
                let local = syn::Ident::new(
                    &format!("__compact_recorded_discarded_field_{next_temp}"),
                    Span::call_site(),
                );
                next_temp += 1;
                return_steps.push(syn::parse_quote!(let #local: runtime::Field = #value;));
                locals.insert(binding.name.clone(), syn::parse_quote!(#local));
            }
            (return_steps, syn::parse_quote!(()))
        }
        StateReturn::Expression {
            value: Expr::Parameter { name },
        } if recorded_root_return
            .as_ref()
            .is_some_and(|(bound_name, _)| *bound_name == name.as_str()) =>
        {
            (
                Vec::new(),
                recorded_root_return.expect("matching root local").1,
            )
        }
        StateReturn::Unit if circuit.result == Type::Unit => {
            if steps.is_empty() {
                return Ok(RecordingOutcome::Unsupported(RecordingGap::no_effect()));
            }
            (Vec::new(), syn::parse_quote!(()))
        }
        StateReturn::Expression {
            value: Expr::FieldToBytes32 { value },
        } if circuit.actions.is_empty() && circuit.result == (Type::Bytes { length: 32 }) => {
            let Expr::FieldCast { value } = value.as_ref() else {
                return Ok(RecordingOutcome::Unsupported(RecordingGap::returned(
                    &circuit.return_value,
                )));
            };
            let Expr::CounterRead { field, index } = value.as_ref() else {
                return Ok(RecordingOutcome::Unsupported(RecordingGap::returned(
                    &circuit.return_value,
                )));
            };
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            if declaration.declaration != LedgerFieldKind::Counter || declaration.index != *index {
                return Ok(RecordingOutcome::Unsupported(RecordingGap::returned(
                    &circuit.return_value,
                )));
            }
            let slot = ident(field)?;
            let converted =
                crate::field_to_bytes_32_syntax(syn::parse_quote!(runtime::Field::from(observed)));
            (
                vec![syn::parse_quote!(
                    let (frame, observed): (_, u64) =
                        crate::ledger_slots::#slot.record_read(frame)?;
                )],
                converted,
            )
        }
        StateReturn::CounterRead { field, index }
            if circuit.result
                == (Type::Unsigned {
                    max: u64::MAX.to_string(),
                }) =>
        {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            if declaration.declaration != LedgerFieldKind::Counter || declaration.index != *index {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            let slot = ident(field)?;
            (
                vec![syn::parse_quote!(
                    let (frame, observed): (_, u64) =
                        crate::ledger_slots::#slot.record_read(frame)?;
                )],
                syn::parse_quote!(
                    runtime::BoundedUint::<18446744073709551615>::new(observed as u128)
                        .expect("ledger Counter fits Uint<64>")
                ),
            )
        }
        StateReturn::CellRead { field, index } if recordable_cell_type(&circuit.result) => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            if declaration.declaration
                != (LedgerFieldKind::Cell {
                    ty: circuit.result.clone(),
                })
                || declaration.index != *index
            {
                return Ok(RecordingOutcome::Unsupported(RecordingGap::returned(
                    &circuit.return_value,
                )));
            }
            let slot = ident(field)?;
            (
                vec![syn::parse_quote!(
                    let (frame, observed): (_, #result_ty) =
                        crate::ledger_slots::#slot.record_read(frame)?;
                )],
                syn::parse_quote!(observed),
            )
        }
        StateReturn::Expression {
            value: Expr::Call { name, arguments },
        } if circuit.result == Type::Field && !circuit.actions.is_empty() => {
            let Some(callee) = pure_circuits.get(name.as_str()) else {
                return Ok(RecordingOutcome::Unsupported(RecordingGap::returned(
                    &circuit.return_value,
                )));
            };
            if callee.result != Type::Field
                || !closed_pure_field_call(name, pure_circuits, &mut HashSet::new())
            {
                return Ok(RecordingOutcome::Unsupported(RecordingGap::returned(
                    &circuit.return_value,
                )));
            }
            if arguments.len() != callee.parameters.len() {
                return Err(RenderError::ArgumentCount {
                    circuit: name.clone(),
                    expected: callee.parameters.len(),
                    actual: arguments.len(),
                });
            }
            let mut return_steps = Vec::new();
            let mut args = Vec::new();
            for (index, (argument, parameter)) in
                arguments.iter().zip(&callee.parameters).enumerate()
            {
                if parameter.ty != Type::Field {
                    return Ok(RecordingOutcome::Unsupported(RecordingGap::expression(
                        argument,
                        format!("return_value.value.arguments[{index}]"),
                    )));
                }
                let Some(value) = field_expression(
                    argument,
                    &HashMap::new(),
                    &parameters,
                    ledger_fields,
                    witnesses,
                    circuits,
                    shared_callees,
                    &mut return_steps,
                    &mut next_temp,
                    &mut visiting,
                )?
                else {
                    return Ok(RecordingOutcome::Unsupported(RecordingGap::expression(
                        argument,
                        format!("return_value.value.arguments[{index}]"),
                    )));
                };
                let arg = syn::Ident::new(
                    &format!("__compact_recorded_arg_{}", next_temp),
                    Span::call_site(),
                );
                next_temp += 1;
                return_steps.push(syn::parse_quote!(let #arg: runtime::Field = #value;));
                args.push(arg);
            }
            let method = ident(name)?;
            let result = syn::Ident::new(
                &format!("__compact_recorded_pure_return_{}", next_temp),
                Span::call_site(),
            );
            return_steps.push(syn::parse_quote! {
                let #result: runtime::Field = crate::pure_circuits::#method(#(#args),*)?;
            });
            (return_steps, syn::parse_quote!(#result))
        }
        StateReturn::Expression { value }
            if circuit.result == Type::Field
                && circuit.actions.is_empty()
                && (helper || (contains_cell_read(value) && !matches!(value, Expr::If { .. }))) =>
        {
            let mut return_steps = Vec::new();
            let Some(result) = field_expression(
                value,
                &HashMap::new(),
                &parameters,
                ledger_fields,
                witnesses,
                circuits,
                shared_callees,
                &mut return_steps,
                &mut next_temp,
                &mut visiting,
            )?
            else {
                return Ok(RecordingOutcome::Unsupported(RecordingGap::expression(
                    value,
                    "return_value.value".to_owned(),
                )));
            };
            (return_steps, result)
        }
        StateReturn::Expression { value }
            if contains_cell_read(value)
                && circuit.result == Type::Boolean
                && circuit.actions.is_empty()
                && !matches!(value, Expr::If { .. }) =>
        {
            let mut return_steps = Vec::new();
            let Some(result) = boolean_expression(
                value,
                &HashMap::new(),
                &parameters,
                ledger_fields,
                witnesses,
                circuits,
                &mut return_steps,
                &mut next_temp,
                &mut visiting,
            )?
            else {
                return Ok(RecordingOutcome::Unsupported(RecordingGap::expression(
                    value,
                    "return_value.value".to_owned(),
                )));
            };
            (return_steps, result)
        }
        StateReturn::Expression {
            value:
                Expr::If {
                    condition,
                    then,
                    otherwise,
                },
        } => {
            match legacy::returns::conditional::lower(
                condition,
                then,
                otherwise,
                &circuit.result,
                &circuit.return_value,
                &result_ty,
                &parameters,
                ledger_fields,
                witnesses,
                pure_circuits,
                circuits,
                &mut next_temp,
                &mut visiting,
            )? {
                RecordingOutcome::Supported(result) => result,
                RecordingOutcome::Unsupported(gap) => {
                    return Ok(RecordingOutcome::Unsupported(gap));
                }
            }
        }
        StateReturn::SetMember {
            field,
            index,
            value,
        } if circuit.result == Type::Boolean => {
            match legacy::returns::collections::set::member(
                field,
                *index,
                value,
                &circuit.return_value,
                &parameters,
                ledger_fields,
                witnesses,
                circuits,
                &mut next_temp,
                &mut visiting,
            )? {
                RecordingOutcome::Supported(result) => result,
                RecordingOutcome::Unsupported(gap) => {
                    return Ok(RecordingOutcome::Unsupported(gap));
                }
            }
        }
        StateReturn::SetSize { field, index }
            if circuit.result
                == (Type::Unsigned {
                    max: u64::MAX.to_string(),
                }) =>
        {
            match legacy::returns::collections::set::size(
                field,
                *index,
                &circuit.return_value,
                ledger_fields,
            )? {
                RecordingOutcome::Supported(result) => result,
                RecordingOutcome::Unsupported(gap) => {
                    return Ok(RecordingOutcome::Unsupported(gap));
                }
            }
        }
        StateReturn::SetIsEmpty { field, index } if circuit.result == Type::Boolean => {
            match legacy::returns::collections::set::is_empty(
                field,
                *index,
                &circuit.return_value,
                ledger_fields,
            )? {
                RecordingOutcome::Supported(result) => result,
                RecordingOutcome::Unsupported(gap) => {
                    return Ok(RecordingOutcome::Unsupported(gap));
                }
            }
        }
        StateReturn::ListLength { field, index }
            if circuit.result
                == (Type::Unsigned {
                    max: u64::MAX.to_string(),
                }) =>
        {
            match legacy::returns::collections::list::length(
                field,
                *index,
                &circuit.return_value,
                ledger_fields,
            )? {
                RecordingOutcome::Supported(result) => result,
                RecordingOutcome::Unsupported(gap) => {
                    return Ok(RecordingOutcome::Unsupported(gap));
                }
            }
        }
        StateReturn::ListIsEmpty { field, index } if circuit.result == Type::Boolean => {
            match legacy::returns::collections::list::is_empty(
                field,
                *index,
                &circuit.return_value,
                ledger_fields,
            )? {
                RecordingOutcome::Supported(result) => result,
                RecordingOutcome::Unsupported(gap) => {
                    return Ok(RecordingOutcome::Unsupported(gap));
                }
            }
        }
        StateReturn::ListHead { field, index } => {
            match legacy::returns::collections::list::head(
                field,
                *index,
                &circuit.return_value,
                &circuit.result,
                &result_ty,
                ledger_fields,
            )? {
                RecordingOutcome::Supported(result) => result,
                RecordingOutcome::Unsupported(gap) => {
                    return Ok(RecordingOutcome::Unsupported(gap));
                }
            }
        }
        StateReturn::MapMember { field, index, key }
        | StateReturn::MapLookup { field, index, key } => {
            match legacy::returns::collections::map::member_or_lookup(
                field,
                *index,
                key,
                &circuit.return_value,
                &circuit.result,
                &result_ty,
                &parameters,
                ledger_fields,
                witnesses,
                circuits,
                &mut next_temp,
                &mut visiting,
            )? {
                RecordingOutcome::Supported(result) => result,
                RecordingOutcome::Unsupported(gap) => {
                    return Ok(RecordingOutcome::Unsupported(gap));
                }
            }
        }
        StateReturn::MapSize { field, index }
            if circuit.result
                == (Type::Unsigned {
                    max: u64::MAX.to_string(),
                }) =>
        {
            match legacy::returns::collections::map::size(
                field,
                *index,
                &circuit.return_value,
                ledger_fields,
            )? {
                RecordingOutcome::Supported(result) => result,
                RecordingOutcome::Unsupported(gap) => {
                    return Ok(RecordingOutcome::Unsupported(gap));
                }
            }
        }
        StateReturn::MapIsEmpty { field, index } if circuit.result == Type::Boolean => {
            match legacy::returns::collections::map::is_empty(
                field,
                *index,
                &circuit.return_value,
                ledger_fields,
            )? {
                RecordingOutcome::Supported(result) => result,
                RecordingOutcome::Unsupported(gap) => {
                    return Ok(RecordingOutcome::Unsupported(gap));
                }
            }
        }
        StateReturn::MerkleCheckRoot { field, index, root }
        | StateReturn::HistoricMerkleCheckRoot { field, index, root }
            if circuit.result == Type::Boolean && circuit.actions.is_empty() =>
        {
            match legacy::returns::merkle::check_root(
                field,
                *index,
                root,
                &circuit.return_value,
                &parameters,
                ledger_fields,
            )? {
                RecordingOutcome::Supported(result) => result,
                RecordingOutcome::Unsupported(gap) => {
                    return Ok(RecordingOutcome::Unsupported(gap));
                }
            }
        }
        StateReturn::MerkleIsFull { field, index }
        | StateReturn::HistoricMerkleIsFull { field, index }
            if circuit.result == Type::Boolean && circuit.actions.is_empty() =>
        {
            match legacy::returns::merkle::is_full(
                field,
                *index,
                &circuit.return_value,
                ledger_fields,
            )? {
                RecordingOutcome::Supported(result) => result,
                RecordingOutcome::Unsupported(gap) => {
                    return Ok(RecordingOutcome::Unsupported(gap));
                }
            }
        }
        StateReturn::Expression {
            value: Expr::Let { bindings, body },
        } if circuit.result == Type::Boolean
            && circuit.actions.is_empty()
            && bindings.len() == 1
            && matches!(body.as_ref(), Expr::MerkleCheckRoot { .. }) =>
        {
            match legacy::returns::merkle::witnessed_path_root(
                bindings,
                body,
                ledger_fields,
                pure_circuits,
                witnesses,
            )? {
                RecordingOutcome::Supported(result) => result,
                RecordingOutcome::Unsupported(gap) => {
                    return Ok(RecordingOutcome::Unsupported(gap));
                }
            }
        }
        _ => {
            let gap = shielded_send_rejection
                .filter(|gap| gap.code == RecordingGapCode::UnsupportedExpression)
                .unwrap_or_else(|| RecordingGap::returned(&circuit.return_value));
            return Ok(RecordingOutcome::Unsupported(gap));
        }
    };
    if !organizer_gate && steps.is_empty() && return_steps.is_empty() {
        return Ok(RecordingOutcome::Unsupported(RecordingGap::no_effect()));
    }
    if circuit
        .parameters
        .iter()
        .any(|parameter| parameter.ty == Type::OpaqueString)
        && !organizer_gate
    {
        return Ok(RecordingOutcome::Unsupported(
            match circuit.actions.first() {
                Some(action) => RecordingGap::action(action, "actions[0]".to_owned()),
                None => RecordingGap::returned(&circuit.return_value),
            },
        ));
    }

    let uses_witness = circuit_uses_witness(circuit, circuits, &mut HashSet::new())?;
    let item = item::assemble(
        circuit,
        circuits,
        shared_callees,
        helper,
        uses_witness,
        &name,
        &args,
        &steps,
        &return_steps,
        &result,
        &result_ty,
    )?;
    Ok(RecordingOutcome::Supported(item))
}
