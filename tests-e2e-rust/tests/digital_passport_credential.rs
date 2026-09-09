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

//
// Executing Rust↔TS parity gate for the vendored digital-passport dogfood
// contract (examples/dogfood/digital-passport-credential/ — third-party
// enclave, see its PROVENANCE.md).
//
// codegen_regression pins the generated TEXT byte-for-byte; this test pins
// BEHAVIOR against the committed TypeScript reference capture
// (fixtures/digital-passport-credential-ts-state.json, produced by
// fixtures/capture-digital-passport-credential.mjs against the TS backend):
//
//   1. derived_values_reproduce_ts_reference — every commitment, root, and
//      proof challenge the Rust pure circuits re-derive from the recorded
//      fixture inputs must equal the captured TS bytes, hex for hex.
//   2. jubjub_keys_reproduce_ts_reference — the fixture's public keys and
//      signature announcements re-derived via ec_mul_generator must equal
//      the captured TS points (curve arithmetic parity).
//   3. age_predicate_cases_match_ts_reference — every civil-date-helper
//      ternary site and assert-fail path (driven through the exported
//      age-predicate circuit, the helper's only call site) plus the
//      calendar-semantics cases must produce the exact captured outcome,
//      including the exact `failed assert:` message.
//   4. protocol_roundtrip_matches_ts_reference — the full
//      issuance/presentation/verification round-trip over the
//      upstream-shaped fixture (real Jubjub keys and real proofs; the
//      result and submission validators verify them) plus the two tamper
//      rejections must match the captured outcomes step by step.
//
// The fixture construction (keys, labels, scalars, openings) is a faithful
// port of upstream src/testing/ at the pinned rev; every input value the
// Rust side cannot re-derive through the circuits is replayed verbatim
// from the JSON capture.

use compact_contract_digital_passport_credential::pure_circuits;
use compact_contract_digital_passport_credential::{
    ContractAddress, Credential, CredentialProtocolFeatures, DigitalPassportCivilDate,
    DigitalPassportClaimCommitments, DigitalPassportClaimValues,
    DigitalPassportCredentialPrivateParts, DigitalPassportDisclosures,
    DigitalPassportIssuanceOfferBody, DigitalPassportIssuanceRequestBody,
    DigitalPassportIssuanceResultBody, DigitalPassportOpenings, DigitalPassportPresentationRequest,
    DigitalPassportVerificationRequestBody, DigitalPassportVerificationResultBody,
    DigitalPassportVerificationSubmissionBody, ExplicitHolderBinding, HolderBindingProfile,
    OfferMessage, Presentation, Proof, ProtocolMessageEnvelope, RequestMessage, RequestMessage_1,
    ResultMessage, ResultMessage_1, SchemaRef, Signature, SubmissionMessage, VerificationMethodRef,
};
use midnight_compact_runtime::{
    construct_jubjub_point, ec_mul_generator, CompactError, Fr, JubjubPoint,
};
use serde::Deserialize;

// ===========================================================================
// TS reference capture shape (mirrors capture-digital-passport-credential.mjs)
// ===========================================================================

#[derive(Deserialize, Debug)]
struct TsReference {
    #[serde(rename = "fixtureValues")]
    fixture_values: FixtureValues,
    #[serde(rename = "derivedValues")]
    derived_values: DerivedValues,
    #[serde(rename = "agePredicateCases")]
    age_predicate_cases: Vec<AgePredicateCase>,
    #[serde(rename = "protocolMessages")]
    protocol_messages: ProtocolMessages,
    #[serde(rename = "protocolCases")]
    protocol_cases: Vec<ProtocolCase>,
}

#[derive(Deserialize, Debug)]
struct FixtureValues {
    issuer: Participant,
    holder: Participant,
    #[serde(rename = "issuerNonce")]
    issuer_nonce: u64,
    #[serde(rename = "holderNonce")]
    holder_nonce: u64,
    #[serde(rename = "credentialProof")]
    credential_proof: ProofRef,
    #[serde(rename = "presentationProof")]
    presentation_proof: ProofRef,
    #[serde(rename = "verifierChallengeHex")]
    verifier_challenge_hex: String,
    schema: SchemaRefRef,
    #[serde(rename = "claimValues")]
    claim_values: ClaimValuesRef,
    openings: OpeningsRef,
    credential: CredentialMeta,
    presentation: PresentationFlags,
    #[serde(rename = "presentationRequest")]
    presentation_request: PresentationRequestFlags,
    #[serde(rename = "noProtocolResponseReferenceHex")]
    no_protocol_response_reference_hex: String,
    #[serde(rename = "normalizedPresentationRequestRootHex")]
    normalized_presentation_request_root_hex: String,
}

#[derive(Deserialize, Debug)]
struct Participant {
    #[serde(rename = "secretKeyHex")]
    secret_key_hex: String,
    #[serde(rename = "publicKey")]
    public_key: PointRef,
    #[serde(rename = "didContractAddressHex")]
    did_contract_address_hex: String,
    #[serde(rename = "methodIdHex")]
    method_id_hex: String,
}

#[derive(Deserialize, Debug)]
struct PointRef {
    #[serde(rename = "xHex")]
    x_hex: String,
    #[serde(rename = "yHex")]
    y_hex: String,
}

#[derive(Deserialize, Debug)]
struct ProofRef {
    #[serde(rename = "createdAt")]
    created_at: u64,
    #[serde(rename = "challengeHashHex")]
    challenge_hash_hex: String,
    #[serde(rename = "publicKey")]
    public_key: PointRef,
    r: PointRef,
    #[serde(rename = "sHex")]
    s_hex: String,
}

#[derive(Deserialize, Debug)]
struct SchemaRefRef {
    #[serde(rename = "packageIdHex")]
    package_id_hex: String,
    #[serde(rename = "schemaIdHex")]
    schema_id_hex: String,
}

#[derive(Deserialize, Debug)]
struct ClaimValuesRef {
    #[serde(rename = "firstNameValuePaddedHex")]
    first_name_value_padded_hex: String,
    #[serde(rename = "lastNameValuePaddedHex")]
    last_name_value_padded_hex: String,
    #[serde(rename = "documentNumberValueHex")]
    document_number_value_hex: String,
    #[serde(rename = "issuingStateValueHex")]
    issuing_state_value_hex: String,
}

#[derive(Deserialize, Debug)]
struct OpeningsRef {
    #[serde(rename = "firstNameOpeningHex")]
    first_name_opening_hex: String,
    #[serde(rename = "lastNameOpeningHex")]
    last_name_opening_hex: String,
    #[serde(rename = "dateOfBirthOpeningHex")]
    date_of_birth_opening_hex: String,
    #[serde(rename = "documentNumberOpeningHex")]
    document_number_opening_hex: String,
    #[serde(rename = "issuingStateOpeningHex")]
    issuing_state_opening_hex: String,
}

#[derive(Deserialize, Debug)]
struct CredentialMeta {
    #[serde(rename = "issuedAt")]
    issued_at: u64,
    #[serde(rename = "hasExpiration")]
    has_expiration: bool,
    #[serde(rename = "expiresAt")]
    expires_at: u64,
}

#[derive(Deserialize, Debug)]
struct PresentationFlags {
    #[serde(rename = "revealFirstName")]
    reveal_first_name: bool,
    #[serde(rename = "revealLastName")]
    reveal_last_name: bool,
    #[serde(rename = "revealDocumentNumber")]
    reveal_document_number: bool,
    #[serde(rename = "revealIssuingState")]
    reveal_issuing_state: bool,
}

#[derive(Deserialize, Debug)]
struct PresentationRequestFlags {
    #[serde(rename = "requireFirstNameDisclosure")]
    require_first_name_disclosure: bool,
    #[serde(rename = "requireLastNameDisclosure")]
    require_last_name_disclosure: bool,
    #[serde(rename = "requireAgeOverThreshold")]
    require_age_over_threshold: bool,
    #[serde(rename = "requestedAgeThresholdYears")]
    requested_age_threshold_years: u8,
    #[serde(rename = "requireDocumentNumberDisclosure")]
    require_document_number_disclosure: bool,
    #[serde(rename = "requireIssuingStateDisclosure")]
    require_issuing_state_disclosure: bool,
}

#[derive(Deserialize, Debug)]
struct DerivedValues {
    #[serde(rename = "firstNameCommitmentHex")]
    first_name_commitment_hex: String,
    #[serde(rename = "lastNameCommitmentHex")]
    last_name_commitment_hex: String,
    #[serde(rename = "dateOfBirthCommitmentHex")]
    date_of_birth_commitment_hex: String,
    #[serde(rename = "documentNumberCommitmentHex")]
    document_number_commitment_hex: String,
    #[serde(rename = "issuingStateCommitmentHex")]
    issuing_state_commitment_hex: String,
    #[serde(rename = "claimRootHex")]
    claim_root_hex: String,
    #[serde(rename = "credentialBodyRootHex")]
    credential_body_root_hex: String,
    #[serde(rename = "presentationBodyRootHex")]
    presentation_body_root_hex: String,
    #[serde(rename = "presentationRequestBodyRootHex")]
    presentation_request_body_root_hex: String,
    #[serde(rename = "issuanceProofChallengeHex")]
    issuance_proof_challenge_hex: String,
    #[serde(rename = "presentationProofChallengeHex")]
    presentation_proof_challenge_hex: String,
}

#[derive(Deserialize, Debug)]
struct AgePredicateCase {
    name: String,
    #[serde(rename = "dobDays")]
    dob_days: u32,
    #[serde(rename = "currentDay")]
    current_day: u32,
    #[serde(rename = "thresholdYears")]
    threshold_years: u8,
    #[serde(rename = "predicateRequested")]
    predicate_requested: bool,
    #[serde(rename = "dobDate")]
    dob_date: CivilDateRef,
    #[serde(rename = "currentDate")]
    current_date: CivilDateRef,
    #[serde(rename = "dobOpeningHex")]
    dob_opening_hex: String,
    outcome: TsOutcome,
}

#[derive(Deserialize, Debug)]
struct CivilDateRef {
    year: u32,
    month: u32,
    day: u32,
    #[serde(rename = "yearAdjustedQuotient4")]
    year_adjusted_quotient4: u32,
    #[serde(rename = "yearAdjustedQuotient100")]
    year_adjusted_quotient100: u32,
    #[serde(rename = "yearAdjustedQuotient400")]
    year_adjusted_quotient400: u32,
    #[serde(rename = "marchBasedMonthDayOffset")]
    march_based_month_day_offset: u32,
}

#[derive(Deserialize, Debug)]
struct ProtocolMessages {
    features: FeaturesRef,
    #[serde(rename = "issuanceOffer")]
    issuance_offer: IssuanceOfferRef,
    #[serde(rename = "issuanceRequest")]
    issuance_request: IssuanceRequestRef,
    #[serde(rename = "issuanceResult")]
    issuance_result: IssuanceResultRef,
    #[serde(rename = "verificationRequest")]
    verification_request: VerificationRequestRef,
    #[serde(rename = "verificationSubmission")]
    verification_submission: EnvelopeOnlyRef,
    #[serde(rename = "verificationResult")]
    verification_result: VerificationResultRef,
}

#[derive(Deserialize, Debug)]
struct FeaturesRef {
    #[serde(rename = "supportsSelectiveDisclosure")]
    supports_selective_disclosure: bool,
    #[serde(rename = "supportsPredicateProofs")]
    supports_predicate_proofs: bool,
    #[serde(rename = "supportsVerifierScopedPseudonym")]
    supports_verifier_scoped_pseudonym: bool,
    #[serde(rename = "supportsSameHolderProof")]
    supports_same_holder_proof: bool,
}

#[derive(Deserialize, Debug)]
struct EnvelopeRef {
    version: u16,
    #[serde(rename = "messageIdHex")]
    message_id_hex: String,
    #[serde(rename = "threadIdHex")]
    thread_id_hex: String,
    #[serde(rename = "initialMessage")]
    initial_message: bool,
    #[serde(rename = "respondsToMessageIdHex")]
    responds_to_message_id_hex: String,
    #[serde(rename = "createdAt")]
    created_at: u64,
    #[serde(rename = "hasExpiresAt")]
    has_expires_at: bool,
    #[serde(rename = "expiresAt")]
    expires_at: u64,
}

#[derive(Deserialize, Debug)]
struct IssuanceOfferRef {
    envelope: EnvelopeRef,
    #[serde(rename = "supportsExpiration")]
    supports_expiration: bool,
    #[serde(rename = "defaultExpirationDays")]
    default_expiration_days: u16,
    #[serde(rename = "requiresHolderPublicKey")]
    requires_holder_public_key: bool,
}

#[derive(Deserialize, Debug)]
struct IssuanceRequestRef {
    envelope: EnvelopeRef,
    #[serde(rename = "holderChallengeHashHex")]
    holder_challenge_hash_hex: String,
    #[serde(rename = "requestExpiration")]
    request_expiration: bool,
    #[serde(rename = "requestedExpirationDays")]
    requested_expiration_days: u16,
}

#[derive(Deserialize, Debug)]
struct IssuanceResultRef {
    envelope: EnvelopeRef,
    #[serde(rename = "issuanceChallengeHashHex")]
    issuance_challenge_hash_hex: String,
}

#[derive(Deserialize, Debug)]
struct VerificationRequestRef {
    envelope: EnvelopeRef,
    #[serde(rename = "verifierChallengeHashHex")]
    verifier_challenge_hash_hex: String,
    #[serde(rename = "requireFirstNameDisclosure")]
    require_first_name_disclosure: bool,
    #[serde(rename = "requireLastNameDisclosure")]
    require_last_name_disclosure: bool,
    #[serde(rename = "requireAgeOverThreshold")]
    require_age_over_threshold: bool,
    #[serde(rename = "requestedAgeThresholdYears")]
    requested_age_threshold_years: u8,
    #[serde(rename = "requireDocumentNumberDisclosure")]
    require_document_number_disclosure: bool,
    #[serde(rename = "requireIssuingStateDisclosure")]
    require_issuing_state_disclosure: bool,
}

#[derive(Deserialize, Debug)]
struct EnvelopeOnlyRef {
    envelope: EnvelopeRef,
    #[serde(rename = "challengeHashHex")]
    challenge_hash_hex: String,
}

#[derive(Deserialize, Debug)]
struct VerificationResultRef {
    envelope: EnvelopeRef,
    approved: bool,
    #[serde(rename = "verifiedThresholdYears")]
    verified_threshold_years: u8,
}

#[derive(Deserialize, Debug)]
struct ProtocolCase {
    name: String,
    outcome: TsOutcome,
}

/// One captured outcome: `"ok"` or `{ "assertionFailed": "<message>" }`.
#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
enum TsOutcome {
    Pass(String),
    AssertionFailed {
        #[serde(rename = "assertionFailed")]
        message: String,
    },
}

// ===========================================================================
// Decoding helpers
// ===========================================================================

fn load_reference() -> TsReference {
    let raw = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fixtures/digital-passport-credential-ts-state.json"
    ))
    .expect("read digital-passport-credential-ts-state.json");
    serde_json::from_str(&raw).expect("parse digital-passport-credential-ts-state.json")
}

fn bytes<const N: usize>(hex_str: &str) -> [u8; N] {
    let decoded = hex::decode(hex_str).expect("decode hex");
    let mut out = [0u8; N];
    out.copy_from_slice(&decoded);
    out
}

/// Parse a captured 256-bit scalar: the JSON hex string is the hex of the
/// scalar's LITTLE-ENDIAN bytes, exactly what `Fr::from_le_bytes` takes.
fn fr_from_le_hex(hex_str: &str) -> Fr {
    let le = hex::decode(hex_str).expect("decode scalar hex");
    Fr::from_le_bytes(&le).expect("scalar is a valid Fr")
}

fn fr_to_le_hex(fr: &Fr) -> String {
    hex::encode(fr.as_le_bytes())
}

fn point(p: &PointRef) -> JubjubPoint {
    construct_jubjub_point(fr_from_le_hex(&p.x_hex), fr_from_le_hex(&p.y_hex))
}

/// Compare a Rust outcome with the captured TS outcome. A passing TS case
/// must yield `Ok(())`; a failing one must yield
/// `Err(CompactError::AssertionFailed(msg))` with the exact captured message
/// — the TS backend surfaces the same message as
/// `CompactError("failed assert: <msg>")`.
fn assert_outcome_matches_ts(result: Result<(), CompactError>, expected: &TsOutcome, name: &str) {
    match expected {
        TsOutcome::Pass(tag) => {
            assert_eq!(tag, "ok", "capture for {name} must be the literal ok");
            assert!(result.is_ok(), "{name}: expected Ok, got {result:?}");
        }
        TsOutcome::AssertionFailed { message } => match result {
            Ok(()) => panic!("{name}: expected AssertionFailed({message:?}), got Ok"),
            Err(CompactError::AssertionFailed(msg)) => {
                assert_eq!(
                    &msg, message,
                    "{name}: assertion message differs from TS reference"
                );
            }
            Err(other) => panic!("{name}: expected AssertionFailed({message:?}), got {other:?}",),
        },
    }
}

// ===========================================================================
// Fixture reconstruction (port of upstream src/testing/credential-fixtures.ts)
// ===========================================================================

struct Fixture {
    reference: TsReference,
    schema: SchemaRef,
    issuer_vmr: VerificationMethodRef,
    holder_vmr: VerificationMethodRef,
    holder_binding: ExplicitHolderBinding,
}

impl Fixture {
    fn load() -> Self {
        let reference = load_reference();
        let fv = &reference.fixture_values;
        let schema = SchemaRef {
            packageId: bytes(&fv.schema.package_id_hex),
            schemaId: bytes(&fv.schema.schema_id_hex),
            majorVersion: 1,
            minorVersion: 0,
        };
        let vmr = |p: &Participant| VerificationMethodRef {
            didContractAddress: ContractAddress {
                bytes: bytes(&p.did_contract_address_hex),
            },
            methodId: bytes(&p.method_id_hex),
        };
        let issuer_vmr = vmr(&fv.issuer);
        let holder_vmr = vmr(&fv.holder);
        Fixture {
            schema,
            holder_binding: ExplicitHolderBinding {
                holderVerificationMethodRef: holder_vmr.clone(),
            },
            issuer_vmr,
            holder_vmr,
            reference,
        }
    }

    /// Claim commitments for a credential whose date-of-birth witness is
    /// `dob_days`; every commitment is re-derived through the generated Rust
    /// circuits from the recorded claim values and openings.
    fn claim_commitments(&self, dob_days: u32) -> DigitalPassportClaimCommitments {
        let fv = &self.reference.fixture_values;
        let cv = &fv.claim_values;
        let op = &fv.openings;
        DigitalPassportClaimCommitments {
            firstNameCommitment: pure_circuits::first_name_commitment(
                bytes(&cv.first_name_value_padded_hex),
                bytes(&op.first_name_opening_hex),
            )
            .expect("first_name_commitment"),
            lastNameCommitment: pure_circuits::last_name_commitment(
                bytes(&cv.last_name_value_padded_hex),
                bytes(&op.last_name_opening_hex),
            )
            .expect("last_name_commitment"),
            dateOfBirthCommitment: pure_circuits::date_of_birth_commitment(
                dob_days,
                bytes(&op.date_of_birth_opening_hex),
            )
            .expect("date_of_birth_commitment"),
            documentNumberCommitment: pure_circuits::document_number_commitment(
                bytes(&cv.document_number_value_hex),
                bytes(&op.document_number_opening_hex),
            )
            .expect("document_number_commitment"),
            issuingStateCommitment: pure_circuits::issuing_state_commitment(
                bytes(&cv.issuing_state_value_hex),
                bytes(&op.issuing_state_opening_hex),
            )
            .expect("issuing_state_commitment"),
        }
    }

    fn private_parts(&self, dob_days: u32) -> DigitalPassportCredentialPrivateParts {
        let fv = &self.reference.fixture_values;
        let cv = &fv.claim_values;
        let op = &fv.openings;
        DigitalPassportCredentialPrivateParts {
            claimValues: DigitalPassportClaimValues {
                firstNameValuePadded: bytes(&cv.first_name_value_padded_hex),
                lastNameValuePadded: bytes(&cv.last_name_value_padded_hex),
                dateOfBirthDays: dob_days,
                documentNumberValue: bytes(&cv.document_number_value_hex),
                issuingStateValue: bytes(&cv.issuing_state_value_hex),
            },
            openings: DigitalPassportOpenings {
                firstNameOpening: bytes(&op.first_name_opening_hex),
                lastNameOpening: bytes(&op.last_name_opening_hex),
                dateOfBirthOpening: bytes(&op.date_of_birth_opening_hex),
                documentNumberOpening: bytes(&op.document_number_opening_hex),
                issuingStateOpening: bytes(&op.issuing_state_opening_hex),
            },
        }
    }

    /// The canonical upstream fixture credential (date-of-birth witness =
    /// epoch day 3650), with its claim root re-derived through the circuit.
    fn credential(&self) -> Credential {
        self.credential_with(3650)
    }

    fn credential_with(&self, dob_days: u32) -> Credential {
        let commitments = self.claim_commitments(dob_days);
        Credential {
            version: 1,
            schema: self.schema.clone(),
            issuerVerificationMethodRef: self.issuer_vmr.clone(),
            holderBinding: self.holder_binding.clone(),
            statusBinding: Default::default(),
            issuedAt: self.reference.fixture_values.credential.issued_at,
            hasExpiration: self.reference.fixture_values.credential.has_expiration,
            expiresAt: self.reference.fixture_values.credential.expires_at,
            claims: Default::default(),
            claimRoot: pure_circuits::digital_passport_claim_root(commitments.clone())
                .expect("digital_passport_claim_root"),
            claimCommitments: commitments,
        }
    }

    /// The canonical presentation (threshold 18, last-name disclosure),
    /// with per-case predicate flag / threshold overrides applied by the
    /// age-predicate test below.
    fn presentation_with(&self, threshold_years: u8, predicate_requested: bool) -> Presentation {
        let fv = &self.reference.fixture_values;
        let cv = &fv.claim_values;
        let op = &fv.openings;
        Presentation {
            version: 1,
            schema: self.schema.clone(),
            credentialClaimRoot: self.credential().claimRoot,
            issuerVerificationMethodRef: self.issuer_vmr.clone(),
            holderBinding: self.holder_binding.clone(),
            disclosed: DigitalPassportDisclosures {
                revealFirstName: fv.presentation.reveal_first_name,
                firstNameValuePadded: [0u8; 64],
                firstNameOpening: [0u8; 32],
                revealLastName: fv.presentation.reveal_last_name,
                lastNameValuePadded: bytes(&cv.last_name_value_padded_hex),
                lastNameOpening: bytes(&op.last_name_opening_hex),
                proveAgeOverThreshold: predicate_requested,
                ageThresholdYears: threshold_years,
                revealDocumentNumber: fv.presentation.reveal_document_number,
                documentNumberValue: [0u8; 32],
                documentNumberOpening: [0u8; 32],
                revealIssuingState: fv.presentation.reveal_issuing_state,
                issuingStateValue: [0u8; 32],
                issuingStateOpening: [0u8; 32],
            },
        }
    }

    fn proof(&self, p: &ProofRef, signer_vmr: VerificationMethodRef) -> Proof {
        Proof {
            signerVerificationMethodRef: signer_vmr,
            createdAt: p.created_at,
            challengeHash: bytes(&p.challenge_hash_hex),
            publicKey: point(&p.public_key),
            signature: Signature {
                r: point(&p.r),
                s: fr_from_le_hex(&p.s_hex),
            },
        }
    }

    fn envelope(e: &EnvelopeRef) -> ProtocolMessageEnvelope {
        ProtocolMessageEnvelope {
            version: e.version,
            messageId: bytes(&e.message_id_hex),
            threadId: bytes(&e.thread_id_hex),
            initialMessage: e.initial_message,
            respondsToMessageId: bytes(&e.responds_to_message_id_hex),
            createdAt: e.created_at,
            hasExpiresAt: e.has_expires_at,
            expiresAt: e.expires_at,
        }
    }

    fn features(f: &FeaturesRef) -> CredentialProtocolFeatures {
        CredentialProtocolFeatures {
            supportsSelectiveDisclosure: f.supports_selective_disclosure,
            supportsPredicateProofs: f.supports_predicate_proofs,
            supportsVerifierScopedPseudonym: f.supports_verifier_scoped_pseudonym,
            supportsSameHolderProof: f.supports_same_holder_proof,
        }
    }
}

/// The Rust outcome of one named protocol step, matched against its capture.
fn run_protocol_step(fixture: &Fixture, name: &str) -> Result<(), CompactError> {
    let fv = &fixture.reference.fixture_values;
    let pm = &fixture.reference.protocol_messages;

    let credential = fixture.credential();
    let presentation = fixture.presentation_with(
        fv.presentation_request.requested_age_threshold_years,
        fv.presentation_request.require_age_over_threshold,
    );
    let credential_proof = fixture.proof(&fv.credential_proof, fixture.issuer_vmr.clone());
    let presentation_proof = fixture.proof(&fv.presentation_proof, fixture.holder_vmr.clone());
    let holder_public_key = point(&fv.holder.public_key);

    let issuance_offer = OfferMessage {
        envelope: Fixture::envelope(&pm.issuance_offer.envelope),
        schema: fixture.schema.clone(),
        issuerVerificationMethodRef: fixture.issuer_vmr.clone(),
        holderBindingProfile: HolderBindingProfile::explicitDid,
        features: Fixture::features(&pm.features),
        body: DigitalPassportIssuanceOfferBody {
            supportsExpiration: pm.issuance_offer.supports_expiration,
            defaultExpirationDays: pm.issuance_offer.default_expiration_days,
            requiresHolderPublicKey: pm.issuance_offer.requires_holder_public_key,
        },
    };

    let issuance_request = RequestMessage_1 {
        envelope: Fixture::envelope(&pm.issuance_request.envelope),
        schema: fixture.schema.clone(),
        issuerVerificationMethodRef: fixture.issuer_vmr.clone(),
        holderBindingProfile: HolderBindingProfile::explicitDid,
        body: DigitalPassportIssuanceRequestBody {
            holderBinding: fixture.holder_binding.clone(),
            holderPublicKey: holder_public_key,
            holderChallengeHash: bytes(&pm.issuance_request.holder_challenge_hash_hex),
            requestExpiration: pm.issuance_request.request_expiration,
            requestedExpirationDays: pm.issuance_request.requested_expiration_days,
        },
    };

    let issuance_result = ResultMessage_1 {
        envelope: Fixture::envelope(&pm.issuance_result.envelope),
        schema: fixture.schema.clone(),
        issuerVerificationMethodRef: fixture.issuer_vmr.clone(),
        holderBindingProfile: HolderBindingProfile::explicitDid,
        body: DigitalPassportIssuanceResultBody {
            credential: credential.clone(),
            credentialProof: credential_proof.clone(),
            holderPublicKey: holder_public_key,
            issuanceChallengeHash: bytes(&pm.issuance_result.issuance_challenge_hash_hex),
            privateParts: fixture.private_parts(3650),
        },
    };

    let verification_request = RequestMessage {
        envelope: Fixture::envelope(&pm.verification_request.envelope),
        schema: fixture.schema.clone(),
        issuerVerificationMethodRef: fixture.issuer_vmr.clone(),
        holderBindingProfile: HolderBindingProfile::explicitDid,
        features: Fixture::features(&pm.features),
        verifierChallengeHash: bytes(&pm.verification_request.verifier_challenge_hash_hex),
        body: DigitalPassportVerificationRequestBody {
            requireFirstNameDisclosure: pm.verification_request.require_first_name_disclosure,
            requireLastNameDisclosure: pm.verification_request.require_last_name_disclosure,
            requireAgeOverThreshold: pm.verification_request.require_age_over_threshold,
            requestedAgeThresholdYears: pm.verification_request.requested_age_threshold_years,
            requireDocumentNumberDisclosure: pm
                .verification_request
                .require_document_number_disclosure,
            requireIssuingStateDisclosure: pm.verification_request.require_issuing_state_disclosure,
        },
    };

    let verification_submission = SubmissionMessage {
        envelope: Fixture::envelope(&pm.verification_submission.envelope),
        schema: fixture.schema.clone(),
        issuerVerificationMethodRef: fixture.issuer_vmr.clone(),
        holderBindingProfile: HolderBindingProfile::explicitDid,
        challengeHash: bytes(&pm.verification_submission.challenge_hash_hex),
        body: DigitalPassportVerificationSubmissionBody {
            credential: credential.clone(),
            credentialProof: credential_proof,
            presentation,
            presentationProof: presentation_proof,
        },
    };

    let verification_result = ResultMessage {
        envelope: Fixture::envelope(&pm.verification_result.envelope),
        approved: pm.verification_result.approved,
        body: DigitalPassportVerificationResultBody {
            credentialRoot: pure_circuits::digital_passport_credential_body_root(
                credential.clone(),
            )
            .expect("digital_passport_credential_body_root"),
            verifiedThresholdYears: pm.verification_result.verified_threshold_years,
        },
    };

    match name {
        "issuance-offer-valid" => {
            pure_circuits::assert_valid_digital_passport_issuance_offer(issuance_offer)
        }
        "issuance-request-valid" => {
            pure_circuits::assert_valid_digital_passport_issuance_request(issuance_request)
        }
        "issuance-request-matches-offer" => {
            pure_circuits::assert_digital_passport_issuance_request_matches_offer(
                issuance_offer,
                issuance_request,
            )
        }
        "issuance-result-valid" => {
            pure_circuits::assert_valid_digital_passport_issuance_result(issuance_result)
        }
        "issuance-result-matches-request" => {
            pure_circuits::assert_digital_passport_issuance_result_matches_request(
                issuance_request,
                issuance_result,
            )
        }
        "verification-request-valid" => {
            pure_circuits::assert_valid_digital_passport_verification_request_message(
                verification_request,
            )
        }
        "verification-submission-valid" => {
            pure_circuits::assert_valid_digital_passport_verification_submission_message(
                verification_submission,
            )
        }
        "verification-submission-matches-request" => {
            pure_circuits::assert_digital_passport_verification_submission_matches_request(
                verification_request,
                verification_submission,
            )
        }
        "verification-result-valid" => {
            pure_circuits::assert_valid_digital_passport_verification_result_message(
                verification_result,
            )
        }
        "verification-result-matches-submission" => {
            pure_circuits::assert_digital_passport_verification_result_matches_submission(
                verification_submission,
                verification_result,
            )
        }
        "issuance-result-challenge-tampered-rejected" => {
            let tampered = ResultMessage_1 {
                body: DigitalPassportIssuanceResultBody {
                    issuanceChallengeHash: [3u8; 32],
                    ..issuance_result.body
                },
                ..issuance_result
            };
            pure_circuits::assert_digital_passport_issuance_result_matches_request(
                issuance_request,
                tampered,
            )
        }
        "verification-challenge-tampered-rejected" => {
            let tampered = RequestMessage {
                verifierChallengeHash: [9u8; 32],
                ..verification_request
            };
            pure_circuits::assert_digital_passport_verification_submission_matches_request(
                tampered,
                verification_submission,
            )
        }
        other => panic!("unknown protocol case in capture: {other}"),
    }
}

// ===========================================================================
// 1. Derived-value byte parity
// ===========================================================================

/// Every commitment, root, and proof challenge the Rust pure circuits
/// re-derive from the captured fixture inputs must equal the captured TS
/// bytes — hex for hex. This pins the persistent-hash-backed derivation
/// circuits (claim commitments, claim root, body roots, context-tagged
/// challenges) against the TS backend, not just against yesterday's Rust.
#[test]
fn digital_passport_derived_values_reproduce_ts_reference() {
    let fixture = Fixture::load();
    let ts = &fixture.reference.derived_values;

    let commitments = fixture.claim_commitments(3650);
    assert_eq!(
        hex::encode(commitments.firstNameCommitment),
        ts.first_name_commitment_hex,
        "firstNameCommitment differs from TS reference"
    );
    assert_eq!(
        hex::encode(commitments.lastNameCommitment),
        ts.last_name_commitment_hex,
        "lastNameCommitment differs from TS reference"
    );
    assert_eq!(
        hex::encode(commitments.dateOfBirthCommitment),
        ts.date_of_birth_commitment_hex,
        "dateOfBirthCommitment differs from TS reference"
    );
    assert_eq!(
        hex::encode(commitments.documentNumberCommitment),
        ts.document_number_commitment_hex,
        "documentNumberCommitment differs from TS reference"
    );
    assert_eq!(
        hex::encode(commitments.issuingStateCommitment),
        ts.issuing_state_commitment_hex,
        "issuingStateCommitment differs from TS reference"
    );

    let claim_root = pure_circuits::digital_passport_claim_root(commitments)
        .expect("digital_passport_claim_root");
    assert_eq!(
        hex::encode(claim_root),
        ts.claim_root_hex,
        "claimRoot differs from TS reference"
    );

    let credential = fixture.credential();
    let credential_body_root =
        pure_circuits::digital_passport_credential_body_root(credential.clone())
            .expect("digital_passport_credential_body_root");
    assert_eq!(
        hex::encode(credential_body_root),
        ts.credential_body_root_hex,
        "credential body root differs from TS reference"
    );

    let presentation = fixture.presentation_with(18, true);
    let presentation_body_root =
        pure_circuits::digital_passport_presentation_body_root(presentation.clone())
            .expect("digital_passport_presentation_body_root");
    assert_eq!(
        hex::encode(presentation_body_root),
        ts.presentation_body_root_hex,
        "presentation body root differs from TS reference"
    );

    let fv = &fixture.reference.fixture_values;
    let presentation_request = DigitalPassportPresentationRequest {
        version: 1,
        schema: fixture.schema.clone(),
        issuerVerificationMethodRef: fixture.issuer_vmr.clone(),
        requireFirstNameDisclosure: fv.presentation_request.require_first_name_disclosure,
        requireLastNameDisclosure: fv.presentation_request.require_last_name_disclosure,
        requireAgeOverThreshold: fv.presentation_request.require_age_over_threshold,
        requestedAgeThresholdYears: fv.presentation_request.requested_age_threshold_years,
        requireDocumentNumberDisclosure: fv.presentation_request.require_document_number_disclosure,
        requireIssuingStateDisclosure: fv.presentation_request.require_issuing_state_disclosure,
        verifierChallengeHash: bytes(&fv.verifier_challenge_hex),
    };
    let request_root =
        pure_circuits::digital_passport_presentation_request_body_root(presentation_request)
            .expect("digital_passport_presentation_request_body_root");
    assert_eq!(
        hex::encode(request_root),
        ts.presentation_request_body_root_hex,
        "presentation-request body root differs from TS reference"
    );

    // The context-tagged challenges: the challenge hash excludes the
    // response scalar s (it hashes the announcement r), so proofs with
    // s = 0 reproduce them — exactly how the fixture signs.
    let issuance_challenge = pure_circuits::issuance_proof_challenge(
        credential_body_root,
        fixture.proof(&fv.credential_proof, fixture.issuer_vmr.clone()),
    )
    .expect("issuance_proof_challenge");
    assert_eq!(
        fr_to_le_hex(&issuance_challenge),
        ts.issuance_proof_challenge_hex,
        "issuance proof challenge differs from TS reference"
    );

    let presentation_challenge = pure_circuits::presentation_proof_challenge(
        presentation_body_root,
        fixture.proof(&fv.presentation_proof, fixture.holder_vmr.clone()),
    )
    .expect("presentation_proof_challenge");
    assert_eq!(
        fr_to_le_hex(&presentation_challenge),
        ts.presentation_proof_challenge_hex,
        "presentation proof challenge differs from TS reference"
    );

    // The no-response sentinel must round-trip too.
    assert_eq!(
        hex::encode(pure_circuits::no_protocol_response_reference().expect("no_response")),
        fv.no_protocol_response_reference_hex,
        "noProtocolResponseReference differs from TS reference"
    );
}

// ===========================================================================
// 2. Jubjub key/announcement parity
// ===========================================================================

/// The fixture's public keys (derived from the recorded secret keys) and
/// signature announcements (derived from the signing nonces — the issuer
/// nonce signs the credential proof, the holder nonce the presentation
/// proof) re-derived via `ec_mul_generator` must equal the captured TS
/// points — pinning the curve arithmetic both fixture builders rely on.
#[test]
fn digital_passport_jubjub_keys_reproduce_ts_reference() {
    let fixture = Fixture::load();
    let fv = &fixture.reference.fixture_values;

    let issuer_pk = ec_mul_generator(fr_from_le_hex(&fv.issuer.secret_key_hex));
    assert_eq!(
        issuer_pk,
        point(&fv.issuer.public_key),
        "issuer public key differs from TS reference"
    );
    assert_eq!(
        ec_mul_generator(Fr::from(fv.issuer_nonce)),
        point(&fv.credential_proof.r),
        "issuer signature announcement differs from TS reference"
    );

    let holder_pk = ec_mul_generator(fr_from_le_hex(&fv.holder.secret_key_hex));
    assert_eq!(
        holder_pk,
        point(&fv.holder.public_key),
        "holder public key differs from TS reference"
    );
    assert_eq!(
        ec_mul_generator(Fr::from(fv.holder_nonce)),
        point(&fv.presentation_proof.r),
        "holder signature announcement differs from TS reference"
    );
}

// ===========================================================================
// 3. Civil-date helper + age-predicate outcomes
// ===========================================================================

/// Every captured civil-date/age-predicate case must produce the exact
/// captured outcome on the Rust side — including the exact assertion
/// message. `assertCivilDateMatchesEpochDays` is internal on both backends,
/// so its ternary sites and fail paths are driven through the exported
/// predicate (their only call site), with distinct messages attributing
/// each failure; the calendar-semantics cases pin the leap-day rules and
/// the `beforeBirthdayThisYear ? 1 : 0` ternary both ways.
#[test]
fn digital_passport_age_predicate_cases_match_ts_reference() {
    let fixture = Fixture::load();

    for case in &fixture.reference.age_predicate_cases {
        let civil_date = |d: &CivilDateRef| DigitalPassportCivilDate {
            year: d.year,
            month: d.month,
            day: d.day,
            yearAdjustedQuotient4: d.year_adjusted_quotient4,
            yearAdjustedQuotient100: d.year_adjusted_quotient100,
            yearAdjustedQuotient400: d.year_adjusted_quotient400,
            marchBasedMonthDayOffset: d.march_based_month_day_offset,
        };

        // The credential is committed to the case's date-of-birth witness
        // (canonical opening); the predicate receives the case's opening.
        let credential = fixture.credential_with(case.dob_days);
        let presentation =
            fixture.presentation_with(case.threshold_years, case.predicate_requested);

        let result = pure_circuits::assert_valid_digital_passport_age_predicate(
            credential,
            presentation,
            case.current_day,
            case.dob_days,
            bytes(&case.dob_opening_hex),
            civil_date(&case.current_date),
            civil_date(&case.dob_date),
        );
        assert_outcome_matches_ts(result, &case.outcome, &case.name);
    }
}

// ===========================================================================
// 4. Protocol round-trip
// ===========================================================================

/// The full issuance → presentation → verification round-trip over the
/// upstream-shaped fixture — including real Jubjub proofs, which the
/// issuance-result and verification-submission validators verify — plus the
/// two tamper rejections, each matching its captured outcome with the exact
/// assertion message. Also pins the protocol→presentation-request
/// normalization: the Rust re-derivation's body root must equal the
/// captured normalized root (and the canonical request's root).
#[test]
fn digital_passport_protocol_roundtrip_matches_ts_reference() {
    let fixture = Fixture::load();

    for case in &fixture.reference.protocol_cases {
        let result = run_protocol_step(&fixture, &case.name);
        assert_outcome_matches_ts(result, &case.outcome, &case.name);
    }

    // Normalization parity: digitalPassportPresentationRequestFromProtocol.
    let fv = &fixture.reference.fixture_values;
    let pm = &fixture.reference.protocol_messages;
    let verification_request = RequestMessage {
        envelope: Fixture::envelope(&pm.verification_request.envelope),
        schema: fixture.schema.clone(),
        issuerVerificationMethodRef: fixture.issuer_vmr.clone(),
        holderBindingProfile: HolderBindingProfile::explicitDid,
        features: Fixture::features(&pm.features),
        verifierChallengeHash: bytes(&pm.verification_request.verifier_challenge_hash_hex),
        body: DigitalPassportVerificationRequestBody {
            requireFirstNameDisclosure: pm.verification_request.require_first_name_disclosure,
            requireLastNameDisclosure: pm.verification_request.require_last_name_disclosure,
            requireAgeOverThreshold: pm.verification_request.require_age_over_threshold,
            requestedAgeThresholdYears: pm.verification_request.requested_age_threshold_years,
            requireDocumentNumberDisclosure: pm
                .verification_request
                .require_document_number_disclosure,
            requireIssuingStateDisclosure: pm.verification_request.require_issuing_state_disclosure,
        },
    };

    let normalized =
        pure_circuits::digital_passport_presentation_request_from_protocol(verification_request)
            .expect("digital_passport_presentation_request_from_protocol");
    let normalized_root =
        pure_circuits::digital_passport_presentation_request_body_root(normalized)
            .expect("digital_passport_presentation_request_body_root");
    assert_eq!(
        hex::encode(normalized_root),
        fv.normalized_presentation_request_root_hex,
        "normalized presentation-request root differs from TS reference"
    );
    assert_eq!(
        hex::encode(normalized_root),
        fixture
            .reference
            .derived_values
            .presentation_request_body_root_hex,
        "normalized presentation-request root differs from the canonical request root"
    );
}
