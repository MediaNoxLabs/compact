// Adapted from testkit-rs/tests/generated_scenarios.rs; externally validated in ADR0324.
use contract::{ledger_contract as cell, runtime as r};
use midnight_compact_testkit::{
    ArtifactIdentity, ContractLab, Environment, LabError, WitnessScript,
};
use r::{
    CompactError, Field,
    context::{ConstructorContext, WitnessContext},
};
use sha2::{Digest, Sha256};

#[derive(Clone)]
struct Private {
    script: WitnessScript<(), Field>,
    calls: u64,
}
struct CellWitness;
impl cell::TryWitnesses<Private> for CellWitness {
    fn fetch_field(
        &self,
        context: WitnessContext<'_, Private, cell::LedgerView<'_>>,
    ) -> Result<(Private, Field), CompactError> {
        let _observed = context.ledger.v()?;
        let mut private = context.private_state.clone();
        let answer = private.script.answer(())?;
        private.calls += 1;
        Ok((private, answer))
    }
}

#[test]
fn recorded_witness_commits_once_and_exhaustion_preserves_checkpoint() {
    let initial = cell::initial_state(ConstructorContext::new(Private {
        script: WitnessScript::new([((), Ok(Field::from(42_u64)))]),
        calls: 0,
    }))
    .unwrap();
    let identity = ArtifactIdentity {
        source_sha256: Sha256::digest(include_bytes!("../witnesses_oracle.compact")).into(),
        generated_sha256: Sha256::digest(include_bytes!("../generated/contract/lib.rs")).into(),
    };
    let environment = Environment::new(Default::default(), Default::default(), [7; 32]);
    let mut lab = ContractLab::from_constructor(identity, environment, initial).unwrap();
    let start = lab.snapshot();
    let mut native = lab.fork();
    let expected = native.native(|ctx| cell::pull(ctx, &CellWitness)).unwrap();
    let report = lab
        .recorded(|ctx| cell::recorded::pull(ctx, &CellWitness))
        .unwrap();
    assert_eq!(report.public_state(), expected.public_state());
    assert_eq!(report.effects(), expected.effects());
    assert_eq!(report.execution_gas(), expected.execution_gas());
    assert!(expected.replay().is_none());
    assert!(report.replay().is_some());
    assert_eq!(
        r::ledger::read_root_cell::<Field, _>(report.public_state().get_ref(), 0).unwrap(),
        Field::from(42_u64)
    );
    assert_eq!(lab.private_state().calls, 1);
    assert_eq!(lab.private_state().script.remaining(), 0);
    assert_eq!(lab.private_state().script.journal(), &[()]);
    let before_failure = lab.snapshot();
    let error = lab
        .recorded(|ctx| cell::recorded::pull(ctx, &CellWitness))
        .unwrap_err();
    assert!(matches!(error, LabError::Execution(_)));
    assert_eq!(
        error.to_string(),
        "circuit execution failed (details redacted)"
    );
    assert_eq!(
        error.execution_error(),
        Some(&CompactError::AssertionFailed(
            "witness script exhausted".into()
        ))
    );
    assert_eq!(lab.snapshot().public_state(), before_failure.public_state());
    assert_eq!(lab.private_state().calls, 1);
    assert_eq!(lab.private_state().script.remaining(), 0);
    assert_eq!(lab.private_state().script.journal(), &[()]);
    lab.restore(&start).unwrap();
    assert_eq!(lab.snapshot().public_state(), start.public_state());
    assert_eq!(lab.private_state().calls, 0);
    assert_eq!(lab.private_state().script.remaining(), 1);
    assert!(lab.private_state().script.journal().is_empty());
}
