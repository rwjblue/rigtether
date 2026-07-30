//! Firmware boundary integration tests.

use rigtether_elecraft_cat::Profile;
use rigtether_elecraft_cat::simulator::{KX2_DOCUMENT_FIXTURE, Simulator};
use rigtether_firmware_core::audio::{
    AudioTelemetry, CONFIGURATION_DESCRIPTOR, USB_BYTES_PER_FRAME, capture_left24_to_mono16,
    playback_mono16_to_stereo24,
};
use rigtether_firmware_core::radio::TypedRadio;
use serde_json::json;

#[test]
fn exact_uac1_descriptor_and_conversion_are_fixed() {
    assert_eq!(CONFIGURATION_DESCRIPTOR.len(), 192);
    assert_eq!(
        u16::from_le_bytes([CONFIGURATION_DESCRIPTOR[2], CONFIGURATION_DESCRIPTOR[3]]),
        192
    );
    assert_eq!(USB_BYTES_PER_FRAME, 96);
    assert_eq!(playback_mono16_to_stereo24(-32_768), [-8_388_608; 2]);
    assert_eq!(playback_mono16_to_stereo24(32_767), [8_388_352; 2]);
    assert_eq!(capture_left24_to_mono16(8_388_607), 32_767);
    assert_eq!(capture_left24_to_mono16(-8_388_608), -32_768);
}

#[test]
fn audio_faults_remain_individually_observable() {
    let healthy = AudioTelemetry {
        playback_fill: 8,
        capture_fill: 8,
        underruns: 0,
        overruns: 0,
        missed_dma: 0,
        sample_corrections: 0,
        i2s_clock_running: true,
    };
    assert!(healthy.healthy());
    assert!(
        !AudioTelemetry {
            sample_corrections: 1,
            ..healthy
        }
        .healthy()
    );
    assert!(
        !AudioTelemetry {
            playback_fill: 1,
            ..healthy
        }
        .healthy()
    );
}

#[test]
fn typed_cat_fixture_is_used_without_raw_escape_hatch() {
    let simulator = Simulator::from_fixture(KX2_DOCUMENT_FIXTURE).expect("fixture");
    let mut radio = TypedRadio::new(Profile::Kx2, simulator);
    let normalized = radio.execute(&json!({"type": "radio_session_normalize"}));
    assert!(normalized.ok);
    assert_eq!(
        normalized.result,
        Some(json!({
            "type": "radio_session_normalize",
            "auto_information": "off",
            "k2_mode": "off",
            "k3_extended_mode": "off"
        }))
    );
    assert_eq!(radio.cat().io().attempts().len(), 6);

    let attempts = radio.cat().io().attempts().len();
    let raw = radio.execute(&json!({"type": "raw_cat", "command": "TX;"}));
    assert!(!raw.ok);
    assert_eq!(raw.error_code, Some("unsupported_radio_operation"));
    assert!(!raw.radio_io_attempted);
    assert!(!raw.release_and_lockout);
    assert_eq!(radio.cat().io().attempts().len(), attempts);

    let keying = radio.execute(&json!({"type": "radio_tx_set", "enabled": true}));
    assert!(!keying.ok);
    assert_eq!(keying.error_code, Some("unsupported_radio_operation"));
    assert!(!keying.radio_io_attempted);
    assert_eq!(radio.cat().io().attempts().len(), attempts);
}

#[test]
fn typed_set_is_query_verified_and_cat_uncertainty_routes_to_lockout() {
    let set_fixture = "\
# provenance=document-derived
# profile=kx2
# source=https://ftp.elecraft.com/KX2/Manuals%20Downloads/K3S%26K3%26KX3%26KX2%20Pgmrs%20Ref%2C%20G5.pdf
500|FA00014060001;|
5|FA;|FA00014060000;
";
    let simulator = Simulator::from_fixture(set_fixture).expect("set fixture");
    let mut radio = TypedRadio::new(Profile::Kx2, simulator);
    let result = radio.execute(&json!({
        "type": "radio_vfo_a_set",
        "frequency_hz": 14_060_001
    }));
    assert!(result.ok);
    assert_eq!(
        result.result,
        Some(json!({
            "type": "radio_vfo_a_set",
            "frequency_hz": 14_060_000,
            "query_verified": true
        }))
    );
    assert_eq!(radio.cat().io().attempts(), &["FA00014060001;", "FA;"]);

    let timeout_fixture = "\
# provenance=document-derived
# profile=kx2
# source=https://ftp.elecraft.com/KX2/Manuals%20Downloads/K3S%26K3%26KX3%26KX2%20Pgmrs%20Ref%2C%20G5.pdf
50|OM;|!timeout
";
    let simulator = Simulator::from_fixture(timeout_fixture).expect("timeout fixture");
    let mut radio = TypedRadio::new(Profile::Kx2, simulator);
    let timeout = radio.execute(&json!({"type": "radio_identify"}));
    assert!(!timeout.ok);
    assert_eq!(timeout.error_code, Some("radio_control_fault"));
    assert!(timeout.radio_io_attempted);
    assert!(timeout.release_and_lockout);
}
