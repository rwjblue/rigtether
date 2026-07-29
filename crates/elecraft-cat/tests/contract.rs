//! End-to-end contract tests for typed CAT and deterministic replay.

use core::time::Duration;

use rigtether_elecraft_cat::simulator::{
    KX2_DOCUMENT_FIXTURE, KX3_DOCUMENT_FIXTURE, Provenance, Simulator, Transcript,
};
use rigtether_elecraft_cat::{
    CatSession, DocumentedFrequencyPolicy, Error, ErrorCode, Exchange, FrameDecoder,
    FrequencyPolicy, Identity, Mode, Operation, OperationResult, OptionFlag, Profile, RadioIo,
    Request, ResponseWindow, SafetyDirective, TxObservation,
};

const POLICY: DocumentedFrequencyPolicy = DocumentedFrequencyPolicy;
type ErrorPredicate = fn(&Error) -> bool;

fn execute_fixture(fixture: &str, profile: Profile) -> CatSession<Simulator> {
    let simulator = Simulator::from_fixture(fixture).expect("fixture parses");
    assert_eq!(simulator.profile(), profile);
    let mut session = CatSession::new(profile, simulator);

    assert_eq!(
        session.execute(Request::Typed(Operation::NormalizeSession), &POLICY),
        Ok(OperationResult::SessionNormalized)
    );
    let identity = session
        .execute(Request::Typed(Operation::Identify), &POLICY)
        .expect("identity parses");
    match (profile, identity) {
        (
            Profile::Kx2,
            OperationResult::Identity(Identity {
                product_code: 1,
                option_flags,
                ..
            }),
        ) => assert!(option_flags.is_empty()),
        (
            Profile::Kx3,
            OperationResult::Identity(Identity {
                product_code: 2,
                option_flags,
                ..
            }),
        ) => assert_eq!(option_flags, vec![OptionFlag::A, OptionFlag::F]),
        value => panic!("unexpected identity: {value:?}"),
    }
    assert_eq!(
        session.execute(
            Request::Typed(Operation::ReadFirmware { include_dsp: true }),
            &POLICY
        ),
        Ok(OperationResult::Firmware(
            rigtether_elecraft_cat::Firmware {
                main: "02.37".into(),
                dsp: Some("02.37".into()),
            }
        ))
    );
    assert_eq!(
        session.execute(Request::Typed(Operation::ReadFrequency), &POLICY),
        Ok(OperationResult::Frequency(7_100_000))
    );
    assert_eq!(
        session.execute(
            Request::Typed(Operation::SetFrequency {
                frequency_hz: 14_060_001
            }),
            &POLICY
        ),
        Ok(OperationResult::Frequency(14_060_000))
    );
    assert!(matches!(
        session.execute(
            Request::Typed(Operation::ReadOperatingState),
            &POLICY
        ),
        Ok(OperationResult::OperatingState(state))
            if state.frequency_hz == 14_060_000
                && state.tx_state == TxObservation::Receive
    ));
    assert_eq!(
        session.execute(Request::Typed(Operation::ReadMode), &POLICY),
        Ok(OperationResult::Mode(if profile == Profile::Kx2 {
            Mode::Usb
        } else {
            Mode::Fm
        }))
    );
    assert_eq!(
        session.execute(Request::Typed(Operation::ReadTxState), &POLICY),
        Ok(OperationResult::TxState(TxObservation::Receive))
    );
    session
}

#[test]
fn every_kx2_allowlisted_operation_round_trips() {
    let session = execute_fixture(KX2_DOCUMENT_FIXTURE, Profile::Kx2);
    assert!(session.io().is_complete());
    assert_eq!(
        session.io().attempts(),
        [
            "AI0;",
            "AI;",
            "K20;",
            "K2;",
            "K30;",
            "K3;",
            "OM;",
            "RVM;",
            "RVD;",
            "FA;",
            "FA00014060001;",
            "FA;",
            "IF;",
            "MD;",
            "TQ;",
        ]
    );
}

#[test]
fn every_kx3_allowlisted_operation_round_trips() {
    let session = execute_fixture(KX3_DOCUMENT_FIXTURE, Profile::Kx3);
    assert!(session.io().is_complete());
}

#[test]
fn incremental_decoder_handles_partial_and_concatenated_input() {
    let mut decoder = FrameDecoder::new();
    assert!(
        decoder
            .push(b"FA00007")
            .expect("partial accepted")
            .is_empty()
    );
    assert_eq!(decoder.pending_len(), 7);
    assert_eq!(
        decoder
            .push(b"100000;TQ0;")
            .expect("remaining and concatenated accepted"),
        ["FA00007100000;", "TQ0;"]
    );
    decoder.finish().expect("nothing pending");
}

#[test]
fn session_parses_partial_chunks() {
    let fixture = r"
# provenance=document-derived
# profile=kx2
# source=test:partial
5|FA;|FA00007~100000;
";
    let simulator = Simulator::from_fixture(fixture).expect("fixture parses");
    let mut session = CatSession::new(Profile::Kx2, simulator);
    assert_eq!(
        session.execute(Request::Typed(Operation::ReadFrequency), &POLICY),
        Ok(OperationResult::Frequency(7_100_000))
    );
}

#[test]
fn ordinary_queries_accept_the_documented_delayed_response() {
    let fixture = r"
# provenance=document-derived
# profile=kx2
# source=test:delayed
200|FA;|FA00007100000;
";
    let simulator = Simulator::from_fixture(fixture).expect("fixture parses");
    let mut session = CatSession::new(Profile::Kx2, simulator);
    assert_eq!(
        session.execute(Request::Typed(Operation::ReadFrequency), &POLICY),
        Ok(OperationResult::Frequency(7_100_000))
    );
}

#[test]
fn question_mark_set_reply_is_busy_or_invalid() {
    let fixture = r"
# provenance=document-derived
# profile=kx2
# source=test:set-question-mark
5|FA00007100000;|?;
";
    let simulator = Simulator::from_fixture(fixture).expect("fixture parses");
    let mut session = CatSession::new(Profile::Kx2, simulator);
    assert_eq!(
        session.execute(
            Request::Typed(Operation::SetFrequency {
                frequency_hz: 7_100_000
            }),
            &POLICY
        ),
        Err(Error::BusyOrInvalid)
    );
    assert_eq!(session.io().cursor(), 1);
}

#[test]
fn malformed_unexpected_unsolicited_busy_and_timeout_are_distinct() {
    let cases: [(&str, ErrorPredicate); 7] = [
        ("5|FA;|FA12;", |error: &Error| {
            matches!(error, Error::MalformedResponse { .. })
        }),
        ("5|FA;|MD2;", |error: &Error| {
            matches!(error, Error::UnexpectedResponse { .. })
        }),
        ("5|FA;|FA00007100000;~TQ0;", |error: &Error| {
            matches!(
                error,
                Error::UnsolicitedResponse { actual } if actual == "TQ0;"
            )
        }),
        ("5|FA;|AI0;~FA00007100000;", |error: &Error| {
            matches!(
                error,
                Error::UnsolicitedResponse { actual } if actual == "AI0;"
            )
        }),
        ("5|FA;|?;", |error: &Error| {
            matches!(error, Error::BusyOrInvalid)
        }),
        ("10|FA;|!timeout", |error: &Error| {
            matches!(
                error,
                Error::Timeout {
                    partial_response: false,
                    ..
                }
            )
        }),
        ("10|FA;|!timeout:FA00007", |error: &Error| {
            matches!(
                error,
                Error::Timeout {
                    partial_response: true,
                    ..
                }
            )
        }),
    ];

    for (record, predicate) in cases {
        let fixture = format!(
            "# provenance=document-derived\n# profile=kx2\n# source=test:error\n{record}\n"
        );
        let simulator = Simulator::from_fixture(&fixture).expect("fixture parses");
        let mut session = CatSession::new(Profile::Kx2, simulator);
        let error = session
            .execute(Request::Typed(Operation::ReadFrequency), &POLICY)
            .expect_err("fixture must fail");
        assert!(
            predicate(&error),
            "unexpected error for {record}: {error:?}"
        );
        assert!(error.radio_io_attempted());
        assert_eq!(error.code(), ErrorCode::RadioControlFault);
        assert_eq!(error.safety_directive(), SafetyDirective::ReleaseAndLockout);
    }
}

#[test]
fn malformed_fixed_fields_are_rejected() {
    let cases = [
        ("OM;", "OM --F-------0X;", Operation::Identify),
        (
            "RVM;",
            "RVM2.37;",
            Operation::ReadFirmware { include_dsp: false },
        ),
        (
            "IF;",
            "IF00007100000     +000020 0002000001 ;",
            Operation::ReadOperatingState,
        ),
        ("MD;", "MD0;", Operation::ReadMode),
        ("TQ;", "TQ2;", Operation::ReadTxState),
    ];

    for (command, response, operation) in cases {
        let fixture = format!(
            "# provenance=document-derived\n# profile=kx2\n# source=test:malformed-field\n5|{command}|{response}\n"
        );
        let simulator = Simulator::from_fixture(&fixture).expect("fixture parses");
        let mut session = CatSession::new(Profile::Kx2, simulator);
        assert!(
            matches!(
                session.execute(Request::Typed(operation), &POLICY),
                Err(Error::MalformedResponse { .. })
            ),
            "malformed response was accepted: {response}"
        );
    }
}

#[test]
fn model_and_profile_differences_are_enforced() {
    let wrong_model = r"
# provenance=document-derived
# profile=kx2
# source=test:model
5|OM;|OM ----------02;
";
    let mut session = CatSession::new(
        Profile::Kx2,
        Simulator::from_fixture(wrong_model).expect("fixture parses"),
    );
    assert!(matches!(
        session.execute(Request::Typed(Operation::Identify), &POLICY),
        Err(Error::ModelMismatch {
            expected: Profile::Kx2,
            actual_product_code: 2
        })
    ));

    let kx2_fm = r"
# provenance=document-derived
# profile=kx2
# source=test:kx2-fm
5|MD;|MD4;
";
    let mut session = CatSession::new(
        Profile::Kx2,
        Simulator::from_fixture(kx2_fm).expect("fixture parses"),
    );
    assert!(matches!(
        session.execute(Request::Typed(Operation::ReadMode), &POLICY),
        Err(Error::ProfileMismatch {
            profile: Profile::Kx2,
            ..
        })
    ));

    let kx3_fm = kx2_fm.replace("profile=kx2", "profile=kx3");
    let mut session = CatSession::new(
        Profile::Kx3,
        Simulator::from_fixture(&kx3_fm).expect("fixture parses"),
    );
    assert_eq!(
        session.execute(Request::Typed(Operation::ReadMode), &POLICY),
        Ok(OperationResult::Mode(Mode::Fm))
    );
}

#[test]
fn frequency_set_requires_query_and_accepts_only_documented_normalization() {
    let normalized = r"
# provenance=document-derived
# profile=kx2
# source=test:normalized
500|FA00007100001;|
5|FA;|FA00007100000;
";
    let mut session = CatSession::new(
        Profile::Kx2,
        Simulator::from_fixture(normalized).expect("fixture parses"),
    );
    assert_eq!(
        session.execute(
            Request::Typed(Operation::SetFrequency {
                frequency_hz: 7_100_001
            }),
            &POLICY
        ),
        Ok(OperationResult::Frequency(7_100_000))
    );
    assert_eq!(session.io().cursor(), 2);

    let mismatch = normalized.replace("FA00007100000;", "FA00007100100;");
    let mut session = CatSession::new(
        Profile::Kx2,
        Simulator::from_fixture(&mismatch).expect("fixture parses"),
    );
    assert!(matches!(
        session.execute(
            Request::Typed(Operation::SetFrequency {
                frequency_hz: 7_100_001
            }),
            &POLICY
        ),
        Err(Error::VerificationMismatch {
            requested_hz: 7_100_001,
            confirmed_hz: 7_100_100
        })
    ));
}

#[test]
fn inconsistent_if_and_tq_observations_fault() {
    let fixture = r"
# provenance=document-derived
# profile=kx2
# source=test:inconsistent
5|IF;|IF00007100000     +000000 0002000001 ;
5|TQ;|TQ1;
";
    let simulator = Simulator::from_fixture(fixture).expect("fixture parses");
    let mut session = CatSession::new(Profile::Kx2, simulator);
    assert!(matches!(
        session.read_consistent_operating_state(),
        Err(Error::InconsistentState {
            if_state: TxObservation::Receive,
            tq_state: TxObservation::TransmitOrPseudoTransmit
        })
    ));
}

#[derive(Default)]
struct AttemptCounter {
    attempts: usize,
}

impl RadioIo for AttemptCounter {
    fn exchange(
        &mut self,
        _command: &str,
        _window: ResponseWindow,
    ) -> Result<Exchange, rigtether_elecraft_cat::IoError> {
        self.attempts += 1;
        Ok(Exchange::Complete {
            chunks: Vec::new(),
            elapsed: Duration::ZERO,
        })
    }
}

struct PermissiveFrequencyPolicy;

impl FrequencyPolicy for PermissiveFrequencyPolicy {
    fn validate(&self, _profile: Profile, _requested_hz: u64) -> Result<(), &'static str> {
        Ok(())
    }

    fn verify(
        &self,
        _profile: Profile,
        _requested_hz: u64,
        _confirmed_hz: u64,
    ) -> Result<(), &'static str> {
        Ok(())
    }
}

#[test]
fn every_non_allowlisted_or_keying_operation_is_rejected_before_io() {
    let rejected = [
        Request::RawCat("TX;"),
        Request::Unsupported("TX"),
        Request::Unsupported("RX"),
        Request::Unsupported("xmit_switch"),
        Request::Unsupported("tune_switch"),
        Request::Unsupported("SWT"),
        Request::Unsupported("SWH"),
        Request::Unsupported("KY"),
        Request::Unsupported("keyer_text"),
        Request::Unsupported("text_transmission"),
        Request::Unsupported("power_write"),
        Request::Unsupported("mode_write"),
        Request::Unsupported("VOX_write"),
        Request::Unsupported("menu_write"),
        Request::Unsupported("baud_write"),
        Request::Unsupported("raw_cat"),
    ];
    let mut session = CatSession::new(Profile::Kx2, AttemptCounter::default());
    for request in rejected {
        let error = session
            .execute(request, &POLICY)
            .expect_err("operation must be denied");
        assert!(matches!(error, Error::UnsupportedOperation { .. }));
        assert_eq!(error.code(), ErrorCode::UnsupportedRadioOperation);
        assert!(!error.radio_io_attempted());
        assert_eq!(error.safety_directive(), SafetyDirective::None);
        assert_eq!(session.io().attempts, 0, "radio_io_attempted: false");
    }

    let error = session
        .execute(
            Request::Typed(Operation::SetFrequency {
                frequency_hz: 100_000_000_000,
            }),
            &POLICY,
        )
        .expect_err("oversized FA field must be denied");
    assert!(matches!(error, Error::InvalidArgument { .. }));
    assert_eq!(error.code(), ErrorCode::InvalidArgument);
    assert!(!error.radio_io_attempted());
    assert_eq!(session.io().attempts, 0, "radio_io_attempted: false");

    let error = session
        .execute(
            Request::Typed(Operation::SetFrequency {
                frequency_hz: 100_000_000_000,
            }),
            &PermissiveFrequencyPolicy,
        )
        .expect_err("core must enforce FA width independently of caller policy");
    assert!(matches!(error, Error::InvalidArgument { .. }));
    assert!(!error.radio_io_attempted());
    assert_eq!(session.io().attempts, 0, "radio_io_attempted: false");
}

#[test]
fn simulator_timing_provenance_and_replay_are_deterministic() {
    let transcript = Transcript::parse(KX2_DOCUMENT_FIXTURE).expect("fixture parses");
    assert_eq!(transcript.provenance, Provenance::DocumentDerived);
    assert_eq!(transcript.steps[1].delay, Duration::from_millis(5));
    assert_eq!(transcript.steps[8].delay, Duration::from_millis(100));
    assert_eq!(transcript.steps[10].delay, Duration::from_millis(500));
    assert_eq!(
        ResponseWindow::Delayed.documented_delay(),
        Duration::from_millis(100)
    );
    assert_eq!(
        ResponseWindow::Delayed.deadline(),
        Duration::from_millis(250)
    );

    let mut simulator = Simulator::new(transcript);
    let first = simulator
        .exchange("AI0;", ResponseWindow::Typical)
        .expect("first replay succeeds");
    assert_eq!(
        first,
        Exchange::Complete {
            chunks: Vec::new(),
            elapsed: Duration::ZERO
        }
    );
    assert_eq!(simulator.cursor(), 1);
    simulator.reset();
    assert_eq!(simulator.cursor(), 0);
    assert_eq!(simulator.elapsed(), Duration::ZERO);
    assert!(simulator.attempts().is_empty());
    assert_eq!(
        simulator
            .exchange("AI0;", ResponseWindow::Typical)
            .expect("reset replay succeeds"),
        first
    );
}

#[test]
fn human_capture_provenance_requires_an_explicit_capture_id() {
    let missing = r"
# provenance=human-captured
# profile=kx2
# source=issue-13-artifact
5|FA;|FA00007100000;
";
    assert!(Transcript::parse(missing).is_err());
    let labeled = missing.replace(
        "# source=issue-13-artifact",
        "# source=issue-13-artifact\n# capture-id=bench-run-0001",
    );
    assert_eq!(
        Transcript::parse(&labeled)
            .expect("capture id permits future human fixture")
            .provenance,
        Provenance::HumanCaptured
    );

    let empty_source = r"
# provenance=document-derived
# profile=kx2
# source=
5|FA;|FA00007100000;
";
    assert!(Transcript::parse(empty_source).is_err());
}
