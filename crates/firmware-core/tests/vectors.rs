//! Shared protocol-vector conformance for the firmware simulator.

use std::fs;
use std::path::PathBuf;

use rigtether_firmware_core::framing::{FramingError, fragment, reassemble};
use rigtether_firmware_core::model::{Model, VectorIds};
use rigtether_firmware_core::strict_json::parse_object;
use serde_json::Value;

fn vectors() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../protocol/vectors/v0.json");
    serde_json::from_str(&fs::read_to_string(path).expect("read shared vectors"))
        .expect("parse shared vectors")
}

fn assert_subset(actual: &Value, expected: &Value, path: &str) -> Result<(), String> {
    if let Some(expected_object) = expected.as_object() {
        let actual_object = actual
            .as_object()
            .ok_or_else(|| format!("{path}: expected object, got {actual}"))?;
        for (key, value) in expected_object {
            let child = actual_object
                .get(key)
                .ok_or_else(|| format!("{path}: missing {key}"))?;
            assert_subset(child, value, &format!("{path}.{key}"))?;
        }
    } else if actual != expected {
        return Err(format!("{path}: expected {expected}, got {actual}"));
    }
    Ok(())
}

#[test]
fn consumes_every_shared_safety_and_logical_scenario() {
    let vectors = vectors();
    let ids = VectorIds::from_value(&vectors["ids"]).expect("valid ids");
    let scenarios = vectors["scenarios"].as_array().expect("scenario array");
    assert_eq!(scenarios.len(), 30);
    for scenario in scenarios {
        let name = scenario["id"].as_str().expect("scenario id");
        let mut model =
            Model::from_initial(ids.clone(), scenario.get("initial")).expect("initial state");
        for (index, step) in scenario["steps"]
            .as_array()
            .expect("scenario steps")
            .iter()
            .enumerate()
        {
            model
                .event(step)
                .unwrap_or_else(|error| panic!("{name} step {}: {error}", index + 1));
            if let Some(expected) = step.get("expect") {
                assert_subset(&model.snapshot(), expected, "$")
                    .unwrap_or_else(|error| panic!("{name} step {}: {error}", index + 1));
            }
        }
    }
}

#[test]
fn consumes_runtime_framing_vectors() {
    let vectors = vectors();
    for vector in vectors["framing"].as_array().expect("framing array") {
        let message = vector["message"].as_str().expect("message").as_bytes();
        let frames = fragment(
            message,
            usize::try_from(vector["frame_limit"].as_u64().expect("frame limit")).unwrap(),
            u32::try_from(vector["transfer_id"].as_u64().expect("transfer id")).unwrap(),
            usize::try_from(vector["max_message_bytes"].as_u64().expect("message limit")).unwrap(),
        )
        .expect("valid framing vector");
        let actual = serde_json::json!({
            "payload_lengths": frames.iter().map(|frame| frame.len() - 16).collect::<Vec<_>>(),
            "flags": frames.iter().map(|frame| frame[1]).collect::<Vec<_>>(),
            "offsets": frames.iter().map(|frame| u32::from_be_bytes(frame[12..16].try_into().expect("offset"))).collect::<Vec<_>>(),
            "round_trip": String::from_utf8(reassemble(
                &frames,
                usize::try_from(vector["frame_limit"].as_u64().expect("frame limit")).unwrap(),
                usize::try_from(
                    vector["max_message_bytes"]
                        .as_u64()
                        .expect("message limit"),
                ).unwrap(),
            ).expect("reassembly")).expect("UTF-8")
        });
        assert_subset(&actual, &vector["expect"], "$").unwrap();
    }
}

#[test]
fn consumes_malformed_framing_vectors() {
    let vectors = vectors();
    for vector in vectors["bad_framing"]
        .as_array()
        .expect("bad framing array")
    {
        let message = vector["message"].as_str().expect("message").as_bytes();
        let mut frames = fragment(
            message,
            usize::try_from(vector["frame_limit"].as_u64().expect("frame limit")).unwrap(),
            u32::try_from(vector["transfer_id"].as_u64().expect("transfer id")).unwrap(),
            usize::try_from(vector["max_message_bytes"].as_u64().expect("message limit")).unwrap(),
        )
        .expect("initial framing");
        match vector["mutation"].as_str().expect("mutation") {
            "gap" => {
                let offset = u32::from_be_bytes(frames[1][12..16].try_into().unwrap()) + 1;
                frames[1][12..16].copy_from_slice(&offset.to_be_bytes());
            }
            "nonzero_reserved" => frames[0][3] = 1,
            "unknown_flags" => frames[0][1] |= 0x80,
            "changed_total_length" => {
                frames[1][8..12]
                    .copy_from_slice(&(u32::try_from(message.len()).unwrap() + 1).to_be_bytes());
            }
            "missing_end" => {
                let last = frames.len() - 1;
                frames[last][1] &= !0x02;
            }
            mutation => panic!("unknown mutation {mutation}"),
        }
        let error = reassemble(
            &frames,
            usize::try_from(vector["frame_limit"].as_u64().expect("frame limit")).unwrap(),
            usize::try_from(vector["max_message_bytes"].as_u64().expect("message limit")).unwrap(),
        )
        .expect_err("malformed framing must fail");
        assert_eq!(
            error.as_str(),
            vector["expect_error"].as_str().expect("expected error"),
            "{}",
            vector["id"]
        );
    }
}

#[test]
fn rejects_zero_length_incoming_transfer() {
    let mut frame = vec![0_u8; 16];
    frame[1] = 0x03;
    frame[4..8].copy_from_slice(&1_u32.to_be_bytes());
    assert_eq!(
        reassemble(&[frame], 20, 1024),
        Err(FramingError::EmptyMessage)
    );
}

#[test]
fn rejects_fragment_larger_than_negotiated_limit() {
    let frame = fragment(b"12345", 21, 1, 1024)
        .expect("valid source frame")
        .remove(0);
    assert_eq!(
        reassemble(&[frame], 20, 1024),
        Err(FramingError::FrameTooLarge)
    );
}

#[test]
fn consumes_strict_json_rejection_vectors() {
    let vectors = vectors();
    for vector in vectors["bad_messages"]
        .as_array()
        .expect("bad message array")
    {
        let raw = if let Some(hex) = vector.get("hex").and_then(Value::as_str) {
            assert_eq!(hex, "ff");
            vec![0xff]
        } else {
            vector["message"]
                .as_str()
                .expect("bad message")
                .as_bytes()
                .to_vec()
        };
        assert!(
            parse_object(&raw).is_err(),
            "{} unexpectedly parsed",
            vector["id"]
        );
    }
    assert!(parse_object(br#"{"value":-0}"#).is_err());
}

#[test]
fn exact_reassembled_bytes_drive_idempotency() {
    let vectors = vectors();
    let ids = VectorIds::from_value(&vectors["ids"]).expect("valid ids");
    let mut model = Model::from_initial(ids.clone(), None).expect("initial state");
    let request = br#"{"type":"request","v":{"major":0,"minor":0},"boot_id":"11111111111111111111111111111111","session_id":"44444444444444444444444444444444","op_id":"00000000000000000000000000000001","seq":1,"command":{"type":"status_read"}}"#;
    let first = model.handle_logical(request).expect("first request");
    assert_eq!(first["type"], "response");
    assert_eq!(first["v"], serde_json::json!({"major": 0, "minor": 0}));
    assert_eq!(first["boot_id"], "11111111111111111111111111111111");
    assert_eq!(first["session_id"], "44444444444444444444444444444444");
    assert_eq!(first["op_id"], "00000000000000000000000000000001");
    assert_eq!(first["seq"], 1);
    assert_eq!(first["accepted_at_ms"], 0);
    assert_eq!(first["next_seq"], 2);
    assert_eq!(first["ok"], true);
    assert_eq!(first["result"]["status_seq"], 2);
    let status = model.protocol_status();
    assert_eq!(status["status_seq"], 2);
    assert_eq!(status["health"]["ble_link"], "connected");
    assert_eq!(status["health"]["protocol_session"], "active");
    assert_eq!(status["health"]["host_usb_audio_route"], "healthy");
    assert_eq!(status["ptt"]["commanded"], "inactive");
    assert_eq!(status["ptt"]["ptt_out"], "inactive");
    assert_eq!(status["ptt"]["inhibit"], "closed");
    let duplicate = model.handle_logical(request).expect("exact duplicate");
    assert_eq!(duplicate, first);
    assert_eq!(model.snapshot()["cached_operations"], 1);
    assert_eq!(model.snapshot()["next_seq"], 2);

    let altered = br#"{"type":"request", "v":{"major":0,"minor":0},"boot_id":"11111111111111111111111111111111","session_id":"44444444444444444444444444444444","op_id":"00000000000000000000000000000001","seq":1,"command":{"type":"status_read"}}"#;
    let denial = model
        .handle_logical(altered)
        .expect("altered duplicate denial");
    assert_eq!(denial["error"]["code"], "altered_duplicate");
    assert_eq!(model.snapshot()["safety_state"], "fault_lockout");
    assert_eq!(model.snapshot()["cached_operations"], 1);
}

#[test]
fn raw_request_requires_explicit_session_identity() {
    let vectors = vectors();
    let ids = VectorIds::from_value(&vectors["ids"]).expect("valid ids");
    let mut model = Model::from_initial(ids, None).expect("initial state");
    let request = br#"{"type":"request","v":{"major":0,"minor":0},"boot_id":"11111111111111111111111111111111","op_id":"00000000000000000000000000000001","seq":1,"command":{"type":"status_read"}}"#;
    let denial = model
        .handle_logical(request)
        .expect("missing identity is a protocol denial");
    assert_eq!(denial["error"]["code"], "malformed");
    assert_eq!(denial["error"]["safety_effect"], "lockout");
    assert_eq!(model.snapshot()["next_seq"], 1);
    assert_eq!(model.snapshot()["cached_operations"], 0);
    assert_eq!(model.snapshot()["safety_state"], "fault_lockout");
}

#[test]
fn raw_session_start_requires_explicit_boot_identity() {
    let vectors = vectors();
    let ids = VectorIds::from_value(&vectors["ids"]).expect("valid ids");
    let mut model = Model::from_initial(ids.clone(), Some(&serde_json::json!({"session": false})))
        .expect("initial state");
    let start = br#"{"type":"session_start","client_nonce":"22222222222222222222222222222222","op_id":"33333333333333333333333333333333","select":{"major":0,"minor":0},"required_capabilities":["first_cause_fault_v0","independent_health_v0","ordered_operations","ptt_leases_v0","typed_radio_v0"],"client_rx_frame_limit":185,"client_max_message_bytes":4096}"#;
    let denial = model
        .handle_logical(start)
        .expect("missing boot identity is a protocol denial");
    assert_eq!(denial["error"]["code"], "malformed");
    assert_eq!(denial["error"]["safety_effect"], "none");
    assert_eq!(model.snapshot()["session_id"], Value::Null);

    let mut active_model = Model::from_initial(ids.clone(), None).expect("active initial state");
    let denial = active_model
        .handle_logical(start)
        .expect("malformed active-session start is a protocol denial");
    assert_eq!(denial["error"]["code"], "malformed");
    assert_eq!(denial["error"]["safety_effect"], "lockout");
    assert_eq!(active_model.snapshot()["safety_state"], "fault_lockout");
    assert_eq!(
        active_model.snapshot()["last_release_code"],
        "protocol_fault"
    );

    for malformed in [
        br#"{"type":"session_start","boot_id":"11111111111111111111111111111111","op_id":"33333333333333333333333333333333","select":{"major":0,"minor":0},"required_capabilities":["first_cause_fault_v0","independent_health_v0","ordered_operations","ptt_leases_v0","typed_radio_v0"],"client_rx_frame_limit":185,"client_max_message_bytes":4096}"#
            .as_slice(),
        br#"{"type":"session_start","boot_id":"11111111111111111111111111111111","client_nonce":"22222222222222222222222222222222","select":{"major":0,"minor":0},"required_capabilities":["first_cause_fault_v0","independent_health_v0","ordered_operations","ptt_leases_v0","typed_radio_v0"],"client_rx_frame_limit":185,"client_max_message_bytes":4096}"#
            .as_slice(),
    ] {
        let mut model = Model::from_initial(ids.clone(), None).expect("active initial state");
        let denial = model
            .handle_logical(malformed)
            .expect("missing required start identity is a protocol denial");
        assert_eq!(denial["error"]["code"], "malformed");
        assert_eq!(denial["error"]["safety_effect"], "lockout");
        assert_eq!(model.snapshot()["safety_state"], "fault_lockout");
    }
}

#[test]
fn disconnect_clears_cached_session_start() {
    let vectors = vectors();
    let ids = VectorIds::from_value(&vectors["ids"]).expect("valid ids");
    let mut model = Model::from_initial(ids, Some(&serde_json::json!({"session": false})))
        .expect("pre-session initial state");
    let start = br#"{"type":"session_start","boot_id":"11111111111111111111111111111111","client_nonce":"22222222222222222222222222222222","op_id":"33333333333333333333333333333333","select":{"major":0,"minor":0},"required_capabilities":["first_cause_fault_v0","independent_health_v0","ordered_operations","ptt_leases_v0","typed_radio_v0"],"client_rx_frame_limit":185,"client_max_message_bytes":4096}"#;
    let first = model
        .handle_logical(start)
        .expect("first session negotiation");
    let first_session = first["result"]["session_id"]
        .as_str()
        .expect("first session identity")
        .to_owned();

    model
        .event(&serde_json::json!({"action": "disconnect"}))
        .expect("disconnect");
    model
        .event(&serde_json::json!({"action": "reconnect"}))
        .expect("reconnect");
    let second = model
        .handle_logical(start)
        .expect("replacement session negotiation");
    let second_session = second["result"]["session_id"]
        .as_str()
        .expect("second session identity");
    assert_ne!(second_session, first_session);
    assert_eq!(model.snapshot()["session_id"], second_session);
}

#[test]
fn profile_selection_clears_cached_session_start() {
    let vectors = vectors();
    let ids = VectorIds::from_value(&vectors["ids"]).expect("valid ids");
    let mut model = Model::from_initial(ids, Some(&serde_json::json!({"session": false})))
        .expect("pre-session initial state");
    let start = br#"{"type":"session_start","boot_id":"11111111111111111111111111111111","client_nonce":"22222222222222222222222222222222","op_id":"33333333333333333333333333333333","select":{"major":0,"minor":0},"required_capabilities":["first_cause_fault_v0","independent_health_v0","ordered_operations","ptt_leases_v0","typed_radio_v0"],"client_rx_frame_limit":185,"client_max_message_bytes":4096}"#;
    let first = model
        .handle_logical(start)
        .expect("first session negotiation");
    let first_session = first["result"]["session_id"]
        .as_str()
        .expect("first session identity")
        .to_owned();
    let select = format!(
        "{{\"type\":\"request\",\"v\":{{\"major\":0,\"minor\":0}},\"boot_id\":\"11111111111111111111111111111111\",\"session_id\":\"{first_session}\",\"op_id\":\"00000000000000000000000000000001\",\"seq\":1,\"command\":{{\"type\":\"radio_profile_select\",\"profile\":\"kx3\"}}}}"
    );
    let selected = model
        .handle_logical(select.as_bytes())
        .expect("profile selection response");
    assert_eq!(selected["result"]["session_invalidated"], true);
    assert_eq!(selected["next_seq"], 2);
    assert_eq!(model.snapshot()["session_id"], Value::Null);

    let replacement = model
        .handle_logical(start)
        .expect("replacement session negotiation");
    let replacement_session = replacement["result"]["session_id"]
        .as_str()
        .expect("replacement session identity");
    assert_ne!(replacement_session, first_session);
    assert_eq!(model.snapshot()["session_id"], replacement_session);
}

#[test]
fn reset_clears_lockout_and_cached_session_start() {
    let vectors = vectors();
    let ids = VectorIds::from_value(&vectors["ids"]).expect("valid ids");
    let start = br#"{"type":"session_start","boot_id":"11111111111111111111111111111111","client_nonce":"22222222222222222222222222222222","op_id":"33333333333333333333333333333333","select":{"major":0,"minor":0},"required_capabilities":["first_cause_fault_v0","independent_health_v0","ordered_operations","ptt_leases_v0","typed_radio_v0"],"client_rx_frame_limit":185,"client_max_message_bytes":4096}"#;
    let malformed = br#"{"type":"request","v":{"major":1,"minor":0},"boot_id":"11111111111111111111111111111111","session_id":"00000000000000000000000000004501","op_id":"00000000000000000000000000000001","seq":1,"command":{"type":"status_read"}}"#;

    for action in ["boot", "watchdog"] {
        let mut model =
            Model::from_initial(ids.clone(), Some(&serde_json::json!({"session": false})))
                .expect("pre-session initial state");
        let started = model
            .handle_logical(start)
            .expect("initial session negotiation");
        assert_eq!(
            started["result"]["session_id"],
            "00000000000000000000000000004501"
        );
        let denial = model
            .handle_logical(malformed)
            .expect("protocol lockout response");
        assert_eq!(denial["error"]["safety_effect"], "lockout");
        assert_eq!(model.snapshot()["safety_state"], "fault_lockout");

        model
            .event(&serde_json::json!({"action": action}))
            .expect("simulated reset");
        assert_eq!(model.snapshot()["safety_state"], "receive_safe");
        assert_eq!(model.snapshot()["first_fault_code"], Value::Null);
        assert_eq!(model.snapshot()["first_fault_id"], Value::Null);

        let stale = model
            .handle_logical(start)
            .expect("old start is evaluated against the new boot");
        assert_eq!(stale["error"]["code"], "stale_boot");
        assert_eq!(model.snapshot()["session_id"], Value::Null);
    }
}

#[test]
fn ptt_release_requires_lease_and_reason_fields() {
    let vectors = vectors();
    let ids = VectorIds::from_value(&vectors["ids"]).expect("valid ids");
    let mut model = Model::from_initial(ids, None).expect("initial state");
    let request = br#"{"type":"request","v":{"major":0,"minor":0},"boot_id":"11111111111111111111111111111111","session_id":"44444444444444444444444444444444","op_id":"00000000000000000000000000000001","seq":1,"command":{"type":"ptt_release","intent_id":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}}"#;
    let denial = model
        .handle_logical(request)
        .expect("malformed release is a command denial");
    assert_eq!(denial["error"]["code"], "invalid_argument");
    assert_eq!(denial["error"]["safety_effect"], "none");
}

#[test]
fn ptt_acquire_validates_arguments_before_safety_state() {
    let vectors = vectors();
    let ids = VectorIds::from_value(&vectors["ids"]).expect("valid ids");
    let invalid_commands = [
        serde_json::json!({
            "type": "ptt_acquire",
            "intent_id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        }),
        serde_json::json!({
            "type": "ptt_acquire",
            "intent_id": "not-a-valid-identity",
            "requested_ms": 500
        }),
        serde_json::json!({
            "type": "ptt_acquire",
            "intent_id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "requested_ms": "500"
        }),
    ];
    for command in invalid_commands {
        let mut model = Model::from_initial(ids.clone(), None).expect("initial state");
        model
            .event(&serde_json::json!({
                "action": "request",
                "op_id": "00000000000000000000000000000001",
                "seq": 1,
                "command": command
            }))
            .expect("invalid acquire request");
        assert_eq!(
            model.snapshot()["last_result"]["error"]["code"],
            "invalid_argument"
        );
    }

    let mut model = Model::from_initial(ids, None).expect("initial state");
    for (op_id, seq, command) in [
        (
            "00000000000000000000000000000001",
            1,
            serde_json::json!({
                "type": "ptt_intent_begin",
                "intent_id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            }),
        ),
        (
            "00000000000000000000000000000002",
            2,
            serde_json::json!({
                "type": "ptt_acquire",
                "intent_id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "requested_ms": 500
            }),
        ),
        (
            "00000000000000000000000000000003",
            3,
            serde_json::json!({
                "type": "ptt_acquire",
                "intent_id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "requested_ms": 0
            }),
        ),
    ] {
        model
            .event(&serde_json::json!({
                "action": "request",
                "op_id": op_id,
                "seq": seq,
                "command": command
            }))
            .expect("request");
    }
    assert_eq!(
        model.snapshot()["last_result"]["error"]["code"],
        "invalid_argument"
    );
}

#[test]
fn ptt_intent_validates_arguments_before_safety_state() {
    let vectors = vectors();
    let ids = VectorIds::from_value(&vectors["ids"]).expect("valid ids");

    let mut active_intent = Model::from_initial(ids.clone(), None).expect("initial state");
    active_intent
        .event(&serde_json::json!({
            "action": "request",
            "op_id": "00000000000000000000000000000001",
            "seq": 1,
            "command": {
                "type": "ptt_intent_begin",
                "intent_id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            }
        }))
        .expect("valid intent");
    active_intent
        .event(&serde_json::json!({
            "action": "request",
            "op_id": "00000000000000000000000000000002",
            "seq": 2,
            "command": {"type": "ptt_intent_begin"}
        }))
        .expect("malformed second intent");
    assert_eq!(
        active_intent.snapshot()["last_result"]["error"]["code"],
        "invalid_argument"
    );

    let mut locked = Model::from_initial(ids, None).expect("initial state");
    locked
        .event(&serde_json::json!({
            "action": "fault",
            "code": "radio_control_fault"
        }))
        .expect("fault lockout");
    locked
        .event(&serde_json::json!({
            "action": "request",
            "op_id": "00000000000000000000000000000001",
            "seq": 1,
            "command": {
                "type": "ptt_intent_begin",
                "intent_id": "not-an-identity"
            }
        }))
        .expect("malformed locked intent");
    assert_eq!(
        locked.snapshot()["last_result"]["error"]["code"],
        "invalid_argument"
    );
}

#[test]
fn ptt_renew_validates_arguments_before_lease_lookup() {
    let vectors = vectors();
    let ids = VectorIds::from_value(&vectors["ids"]).expect("valid ids");
    for command in [
        serde_json::json!({
            "type": "ptt_renew",
            "intent_id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "lease_id": "00000000000000000000000000000001"
        }),
        serde_json::json!({
            "type": "ptt_renew",
            "intent_id": "not-an-identity",
            "lease_id": "00000000000000000000000000000001",
            "requested_ms": 500
        }),
        serde_json::json!({
            "type": "ptt_renew",
            "intent_id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "lease_id": "not-a-lease",
            "requested_ms": 500
        }),
        serde_json::json!({
            "type": "ptt_renew",
            "intent_id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "lease_id": "00000000000000000000000000000001",
            "requested_ms": 0
        }),
    ] {
        let mut model = Model::from_initial(ids.clone(), None).expect("initial state");
        model
            .event(&serde_json::json!({
                "action": "request",
                "op_id": "00000000000000000000000000000001",
                "seq": 1,
                "command": command
            }))
            .expect("invalid renew request");
        assert_eq!(
            model.snapshot()["last_result"]["error"]["code"],
            "invalid_argument"
        );
    }

    let mut model = Model::from_initial(ids, None).expect("initial state");
    for (op_id, seq, command) in [
        (
            "00000000000000000000000000000001",
            1,
            serde_json::json!({
                "type": "ptt_intent_begin",
                "intent_id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            }),
        ),
        (
            "00000000000000000000000000000002",
            2,
            serde_json::json!({
                "type": "ptt_acquire",
                "intent_id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "requested_ms": 500
            }),
        ),
    ] {
        model
            .event(&serde_json::json!({
                "action": "request",
                "op_id": op_id,
                "seq": seq,
                "command": command
            }))
            .expect("request");
    }
    model
        .event(&serde_json::json!({"action": "advance", "ms": 500}))
        .expect("lease expiry");
    model
        .event(&serde_json::json!({
            "action": "request",
            "op_id": "00000000000000000000000000000003",
            "seq": 3,
            "command": {
                "type": "ptt_renew",
                "intent_id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "lease_id": "00000000000000000000000000000001"
            }
        }))
        .expect("malformed expired-lease renewal");
    assert_eq!(
        model.snapshot()["last_result"]["error"]["code"],
        "invalid_argument"
    );
}

#[test]
fn deassertion_deadline_survives_repeated_release_requests() {
    let vectors = vectors();
    let ids = VectorIds::from_value(&vectors["ids"]).expect("valid ids");
    let mut model = Model::from_initial(ids, None).expect("initial state");
    for (op_id, seq, command) in [
        (
            "00000000000000000000000000000001",
            1,
            serde_json::json!({
                "type": "ptt_intent_begin",
                "intent_id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            }),
        ),
        (
            "00000000000000000000000000000002",
            2,
            serde_json::json!({
                "type": "ptt_acquire",
                "intent_id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "requested_ms": 500
            }),
        ),
        (
            "00000000000000000000000000000003",
            3,
            serde_json::json!({
                "type": "ptt_release",
                "intent_id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "lease_id": "00000000000000000000000000000001",
                "reason": "operator_release"
            }),
        ),
    ] {
        model
            .event(&serde_json::json!({
                "action": "request",
                "op_id": op_id,
                "seq": seq,
                "command": command
            }))
            .expect("request");
    }
    assert_eq!(model.snapshot()["deassertion_deadline_ms"], 100);
    model
        .event(&serde_json::json!({"action": "advance", "ms": 50}))
        .expect("partial release interval");
    model
        .event(&serde_json::json!({
            "action": "request",
            "op_id": "00000000000000000000000000000004",
            "seq": 4,
            "command": {
                "type": "ptt_release",
                "intent_id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "lease_id": null,
                "reason": "operator_release"
            }
        }))
        .expect("repeated release");
    assert_eq!(model.snapshot()["deassertion_deadline_ms"], 100);
    model
        .event(&serde_json::json!({"action": "advance", "ms": 49}))
        .expect("before deadline");
    assert_eq!(model.snapshot()["first_fault_code"], Value::Null);
    model
        .event(&serde_json::json!({"action": "advance", "ms": 1}))
        .expect("release deadline");
    assert_eq!(model.snapshot()["first_fault_code"], "output_stuck_active");
}

#[test]
fn explicitly_released_lease_is_not_reported_expired() {
    let vectors = vectors();
    let ids = VectorIds::from_value(&vectors["ids"]).expect("valid ids");
    let mut model = Model::from_initial(ids, None).expect("initial state");
    for (op_id, seq, command) in [
        (
            "00000000000000000000000000000001",
            1,
            serde_json::json!({
                "type": "ptt_intent_begin",
                "intent_id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            }),
        ),
        (
            "00000000000000000000000000000002",
            2,
            serde_json::json!({
                "type": "ptt_acquire",
                "intent_id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "requested_ms": 500
            }),
        ),
        (
            "00000000000000000000000000000003",
            3,
            serde_json::json!({
                "type": "ptt_release",
                "intent_id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "lease_id": "00000000000000000000000000000001",
                "reason": "operator_release"
            }),
        ),
        (
            "00000000000000000000000000000004",
            4,
            serde_json::json!({
                "type": "ptt_renew",
                "intent_id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "lease_id": "00000000000000000000000000000001",
                "requested_ms": 500
            }),
        ),
    ] {
        model
            .event(&serde_json::json!({
                "action": "request",
                "op_id": op_id,
                "seq": seq,
                "command": command
            }))
            .expect("request");
    }
    assert_eq!(
        model.snapshot()["last_result"]["error"]["code"],
        "lease_not_found"
    );
}

#[test]
fn expired_lease_history_is_session_scoped() {
    let vectors = vectors();
    let ids = VectorIds::from_value(&vectors["ids"]).expect("valid ids");
    let mut model = Model::from_initial(ids, None).expect("initial state");
    for (op_id, seq, command) in [
        (
            "00000000000000000000000000000001",
            1,
            serde_json::json!({
                "type": "ptt_intent_begin",
                "intent_id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            }),
        ),
        (
            "00000000000000000000000000000002",
            2,
            serde_json::json!({
                "type": "ptt_acquire",
                "intent_id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "requested_ms": 500
            }),
        ),
    ] {
        model
            .event(&serde_json::json!({
                "action": "request",
                "op_id": op_id,
                "seq": seq,
                "command": command
            }))
            .expect("request");
    }
    model
        .event(&serde_json::json!({"action": "advance", "ms": 500}))
        .expect("lease expiry");
    model
        .event(&serde_json::json!({"action": "new_session"}))
        .expect("replacement session");
    model
        .event(&serde_json::json!({
            "action": "request",
            "op_id": "00000000000000000000000000000003",
            "seq": 1,
            "command": {
                "type": "ptt_renew",
                "intent_id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "lease_id": "00000000000000000000000000000001",
                "requested_ms": 500
            }
        }))
        .expect("old lease renewal");
    assert_eq!(
        model.snapshot()["last_result"]["error"]["code"],
        "lease_not_found"
    );
}

#[test]
fn unknown_inhibit_or_ptt_sense_releases_immediately() {
    let vectors = vectors();
    let ids = VectorIds::from_value(&vectors["ids"]).expect("valid ids");
    for (trigger, release, state) in [
        (
            serde_json::json!({
                "action": "set_health",
                "domain": "inhibit",
                "value": "unknown"
            }),
            "inhibit_open",
            "tx_active",
        ),
        (
            serde_json::json!({"action": "sense", "value": "unknown"}),
            "output_failed_to_assert",
            "fault_lockout",
        ),
    ] {
        let mut model = Model::from_initial(ids.clone(), None).expect("initial state");
        for (op_id, seq, command) in [
            (
                "00000000000000000000000000000001",
                1,
                serde_json::json!({
                    "type": "ptt_intent_begin",
                    "intent_id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                }),
            ),
            (
                "00000000000000000000000000000002",
                2,
                serde_json::json!({
                    "type": "ptt_acquire",
                    "intent_id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                    "requested_ms": 500
                }),
            ),
        ] {
            model
                .event(&serde_json::json!({
                    "action": "request",
                    "op_id": op_id,
                    "seq": seq,
                    "command": command
                }))
                .expect("request");
        }
        model.event(&trigger).expect("unsafe input transition");
        assert_eq!(model.snapshot()["commanded"], "inactive");
        assert_eq!(model.snapshot()["last_release_code"], release);
        assert_eq!(model.snapshot()["safety_state"], state);
    }
}

#[test]
fn prohibited_radio_writes_are_rejected_before_io() {
    let vectors = vectors();
    let ids = VectorIds::from_value(&vectors["ids"]).expect("valid ids");
    for command_type in [
        "power_write",
        "VOX_write",
        "mode_write",
        "menu_write",
        "baud_write",
    ] {
        let mut model = Model::from_initial(ids.clone(), None).expect("initial state");
        model
            .event(&serde_json::json!({
                "action": "request",
                "op_id": "00000000000000000000000000000001",
                "seq": 1,
                "command": {"type": command_type}
            }))
            .expect("prohibited radio request");
        assert_eq!(
            model.snapshot()["last_result"]["error"]["code"],
            "unsupported_radio_operation"
        );
        assert_eq!(
            model.snapshot()["last_result"]["error"]["radio_io_attempted"],
            false
        );
        assert_eq!(model.snapshot()["radio_io_count"], 0);
    }
}

#[test]
fn raw_request_requires_exact_version_and_known_envelope() {
    let vectors = vectors();
    let ids = VectorIds::from_value(&vectors["ids"]).expect("valid ids");
    let mut model = Model::from_initial(ids.clone(), None).expect("initial state");
    let wrong_version = br#"{"type":"request","v":{"major":1,"minor":0},"boot_id":"11111111111111111111111111111111","session_id":"44444444444444444444444444444444","op_id":"00000000000000000000000000000001","seq":1,"command":{"type":"status_read"}}"#;
    let denial = model
        .handle_logical(wrong_version)
        .expect("unsupported request version is a protocol denial");
    assert_eq!(denial["error"]["code"], "malformed");
    assert_eq!(model.snapshot()["next_seq"], 1);
    assert_eq!(model.snapshot()["safety_state"], "fault_lockout");

    let mut model = Model::from_initial(ids.clone(), None).expect("initial state");
    let denial = model
        .handle_logical(br#"{"type":"unknown"}"#)
        .expect("unknown current-session envelope is a protocol denial");
    assert_eq!(denial["error"]["code"], "malformed");
    assert_eq!(denial["error"]["safety_effect"], "lockout");
    assert_eq!(model.snapshot()["safety_state"], "fault_lockout");

    let mut model = Model::from_initial(
        ids,
        Some(&serde_json::json!({
            "session": false
        })),
    )
    .expect("pre-session initial state");
    let denial = model
        .handle_logical(br#"{"type":"request","v":{"major":0,"minor":0},"boot_id":"11111111111111111111111111111111","session_id":"44444444444444444444444444444444","op_id":"00000000000000000000000000000001","seq":1,"command":{"type":"status_read"}}"#)
        .expect("pre-session request is denied receive-safely");
    assert_eq!(denial["error"]["code"], "wrong_session");
    assert_eq!(denial["error"]["safety_effect"], "none");
    assert_eq!(model.snapshot()["safety_state"], "receive_safe");

    let malformed = model
        .handle_logical(br#"{"type":"request","v":{"major":0,"minor":0},"boot_id":"11111111111111111111111111111111","session_id":"44444444444444444444444444444444","seq":1,"command":{"type":"status_read"}}"#)
        .expect("malformed pre-session request is denied before session lookup");
    assert_eq!(malformed["error"]["code"], "malformed");
    assert_eq!(malformed["error"]["safety_effect"], "none");
    assert_eq!(model.snapshot()["safety_state"], "receive_safe");
}

#[test]
fn radio_commands_ignore_unknown_optional_fields() {
    let vectors = vectors();
    let ids = VectorIds::from_value(&vectors["ids"]).expect("valid ids");
    let commands = [
        serde_json::json!({
            "type": "radio_profile_select",
            "profile": "kx3",
            "future_extension": {"ignored": true}
        }),
        serde_json::json!({
            "type": "radio_identify",
            "future_extension": {"ignored": true}
        }),
        serde_json::json!({
            "type": "radio_vfo_a_set",
            "frequency_hz": 7_100_000,
            "future_extension": {"ignored": true}
        }),
        serde_json::json!({
            "type": "radio_session_normalize",
            "future_extension": {"ignored": true}
        }),
        serde_json::json!({
            "type": "radio_firmware_read",
            "future_extension": {"ignored": true}
        }),
        serde_json::json!({
            "type": "radio_vfo_a_read",
            "future_extension": {"ignored": true}
        }),
        serde_json::json!({
            "type": "radio_operating_state_read",
            "future_extension": {"ignored": true}
        }),
        serde_json::json!({
            "type": "radio_mode_read",
            "future_extension": {"ignored": true}
        }),
        serde_json::json!({
            "type": "radio_tx_state_read",
            "future_extension": {"ignored": true}
        }),
    ];

    for (index, command) in commands.into_iter().enumerate() {
        let mut model = Model::from_initial(ids.clone(), None).expect("initial state");
        let request = serde_json::json!({
            "type": "request",
            "v": {"major": 0, "minor": 0},
            "boot_id": "11111111111111111111111111111111",
            "session_id": "44444444444444444444444444444444",
            "op_id": format!("{:032x}", index + 1),
            "seq": 1,
            "command": command
        });
        let encoded = serde_json::to_vec(&request).expect("encode request");
        let response = model
            .handle_logical(&encoded)
            .expect("extension-bearing command");
        assert_eq!(response["ok"], true, "{request}");
    }
}

#[test]
fn deeply_nested_unknown_fields_are_accepted_stack_safely() {
    let vectors = vectors();
    let ids = VectorIds::from_value(&vectors["ids"]).expect("valid ids");
    let mut extension = serde_json::json!(null);
    for _ in 0..160 {
        extension = serde_json::json!([extension]);
    }
    let request = serde_json::json!({
        "type": "request",
        "v": {"major": 0, "minor": 0},
        "boot_id": "11111111111111111111111111111111",
        "session_id": "44444444444444444444444444444444",
        "op_id": "00000000000000000000000000000001",
        "seq": 1,
        "command": {
            "type": "status_read",
            "future_extension": extension
        }
    });
    let encoded = serde_json::to_vec(&request).expect("encode nested request");
    assert!(encoded.len() <= 1024);
    let mut model = Model::from_initial(ids, None).expect("initial state");
    let response = model
        .handle_logical(&encoded)
        .expect("stack-safe extension-bearing command");
    assert_eq!(response["ok"], true);
}
