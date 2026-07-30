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
    assert_eq!(reassemble(&[frame], 1024), Err(FramingError::EmptyMessage));
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
}
