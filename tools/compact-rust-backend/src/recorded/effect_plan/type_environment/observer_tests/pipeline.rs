use super::*;
use crate::recorded::effect_plan::{self, DiagnosticPlanError};
use quote::quote;

fn checked_gap(result: Result<effect_plan::CheckedPlan, DiagnosticPlanError>) -> RecordingGap {
    match result {
        Err(DiagnosticPlanError::Unsupported(rejection)) => rejection.into_parts().0,
        _ => panic!("existing source gap before lower/model checks"),
    }
}

#[test]
fn original_construction_phase_forwards_source_failure_and_success_has_identical_plan_syntax() {
    let root = source(enumeration("Empty", &[]));
    let trace = Trace::default();
    let current = checked_gap(effect_plan::check_with_calls_observed(
        &root,
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        Some(&trace),
    ));
    let plain = checked_gap(effect_plan::check_with_calls_observed(
        &root,
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        None,
    ));
    assert_eq!(current, plain);
    assert_eq!(Some(&current), old_env(&root).as_ref().err());
    one(
        &trace,
        RefusalSite::field(Owner::Type(&root.parameters[0].ty), "variants"),
    );
    let root = source(Type::Field);
    let trace = Trace::default();
    let observed = effect_plan::check_with_calls_observed(
        &root,
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        Some(&trace),
    )
    .unwrap();
    let plain = effect_plan::check_with_calls_observed(
        &root,
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        None,
    )
    .unwrap();
    let observed = effect_plan::render(&observed).unwrap();
    let plain = effect_plan::render(&plain).unwrap();
    let (observed_steps, observed_result) = (&observed.steps, &observed.result);
    let (plain_steps, plain_result) = (&plain.steps, &plain.result);
    assert_eq!(
        quote!(#(#observed_steps)* #observed_result).to_string(),
        quote!(#(#plain_steps)* #plain_result).to_string()
    );
    none(&trace);
}
