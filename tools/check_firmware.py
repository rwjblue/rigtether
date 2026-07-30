#!/usr/bin/env python3
"""Validate fixed M1 firmware inputs without claiming hardware behavior."""

from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ERRORS: list[str] = []

EXPECTED_UUIDS = {
    "3dbbf179-78cb-4753-9c4e-092e2a4e1116",
    "686fd375-482c-407b-856d-3a5e757f9f03",
    "f98fec49-c274-40d9-9b4b-38f5cef50e15",
    "7ba61ae1-e6b0-4e3f-a718-fd31315cd876",
    "f9ecbae1-3204-4870-b338-8f220e000585",
}

EXPECTED_DESCRIPTOR = bytes.fromhex(
    """
    09 02 C0 00 03 01 00 80 FA
    09 04 00 00 00 01 01 00 00
    0A 24 01 00 01 46 00 02 01 02
    0C 24 02 01 01 01 00 01 00 00 00 00
    09 24 06 02 01 01 01 00 00
    09 24 03 03 03 06 00 02 00
    0C 24 02 04 03 06 00 01 00 00 00 00
    09 24 06 05 04 01 01 00 00
    09 24 03 06 01 01 00 05 00
    09 04 01 00 00 01 02 00 00
    09 04 01 01 01 01 02 00 00
    07 24 01 01 08 01 00
    0B 24 02 01 01 02 10 01 80 BB 00
    09 05 08 0D 60 00 01 00 00
    07 25 01 00 00 00 00
    09 04 02 00 00 01 02 00 00
    09 04 02 01 01 01 02 00 00
    07 24 01 06 08 01 00
    0B 24 02 01 01 02 10 01 80 BB 00
    09 05 88 0D 60 00 01 00 00
    07 25 01 00 00 00 00
    """
)


def error(message: str) -> None:
    ERRORS.append(message)


def hex_bytes(path: Path, symbol: str) -> bytes:
    text = path.read_text(encoding="utf-8")
    match = re.search(
        rf"{re.escape(symbol)}(?:\s*:\s*)?\[[^\]]+\]\s*=\s*\[(?P<body>.*?)\];"
        rf"|{re.escape(symbol)}\s*\[[^\]]+\]\s*=\s*\{{(?P<c_body>.*?)\}};",
        text,
        re.DOTALL,
    )
    if match is None:
        error(f"{path.relative_to(ROOT)}: missing {symbol}")
        return b""
    body = match["body"] if match["body"] is not None else match["c_body"]
    return bytes(int(value, 16) for value in re.findall(r"0x([0-9A-Fa-f]{2})", body))


c_descriptor = hex_bytes(
    ROOT / "firmware/nrf5340/src/usb_descriptors.c",
    "rt_uac1_configuration_descriptor",
)
rust_descriptor = hex_bytes(
    ROOT / "crates/firmware-core/src/audio.rs",
    "CONFIGURATION_DESCRIPTOR",
)
if c_descriptor != EXPECTED_DESCRIPTOR:
    error("nRF5340 C UAC1 descriptor differs from the fixed 192-byte M1 descriptor")
if rust_descriptor != EXPECTED_DESCRIPTOR:
    error("Rust simulator UAC1 descriptor differs from the fixed 192-byte M1 descriptor")

ble_source = (ROOT / "firmware/nrf5340/src/ble_service.c").read_text(encoding="utf-8")
encoded_uuid_parts = set(
    re.findall(
        r"BT_UUID_128_ENCODE\(0x([0-9a-f]+), 0x([0-9a-f]+), 0x([0-9a-f]+), "
        r"0x([0-9a-f]+), 0x([0-9a-f]+)\)",
        ble_source,
    )
)
actual_uuids = {"-".join(parts) for parts in encoded_uuid_parts}
if actual_uuids != EXPECTED_UUIDS:
    error(f"BLE UUID set differs: {sorted(actual_uuids)}")
required_roles = [
    "BT_GATT_CHRC_READ, BT_GATT_PERM_READ,\n\t\t\t       read_hello",
    "BT_GATT_CHRC_WRITE, BT_GATT_PERM_WRITE,\n\t\t\t       NULL, write_command",
    "BT_GATT_CHRC_INDICATE, BT_GATT_PERM_NONE",
    "BT_GATT_CHRC_READ | BT_GATT_CHRC_NOTIFY",
]
for role in required_roles:
    if role not in ble_source:
        error(f"BLE characteristic role missing or changed: {role.split(',')[0]}")

required_dispatch = [
    "logical_handler(command_transfer.bytes, command_transfer.total",
    "rt_ble_publish_response(logical_response, response_length)",
    "rt_ble_register_logical_handler(rt_protocol_handle_logical)",
]
for dispatch in required_dispatch:
    if dispatch not in ble_source:
        error(f"completed BLE commands are not dispatched through the logical core: {dispatch}")

required_snapshot_fields = [
    "snapshot.state",
    "snapshot.last_release",
    "snapshot.first_fault",
    "snapshot.inputs.ptt_out_active",
    "snapshot.inputs.inhibit_closed",
    "snapshot.inputs.protocol_session",
]
for field in required_snapshot_fields:
    if field not in ble_source:
        error(f"BLE status does not render safety snapshot field: {field}")

protocol_source = (ROOT / "firmware/nrf5340/src/protocol_service.c").read_text(
    encoding="utf-8"
)
for envelope_field in (
    '\\"accepted_at_ms\\":',
    '\\"next_seq\\":',
    '\\"session_id\\":',
    '\\"op_id\\":',
):
    if envelope_field not in protocol_source:
        error(f"nRF logical response is missing v0 envelope field: {envelope_field}")
for protocol_boundary in (
    "extract_top_level_type(request, request_length",
    "validate_no_duplicate_members(request, request_length)",
    "rt_operation_cache_lookup(",
    "rt_operation_cache_store(",
    "rt_protocol_att_limit_changed",
    'strcmp(type, "host_audio_route_report")',
    'strcmp(type, "ptt_intent_begin")',
    'strcmp(type, "ptt_acquire")',
    'strcmp(type, "ptt_renew")',
    'strcmp(type, "ptt_release")',
    'strcmp(type, "safety_recover")',
    'strcmp(type, "radio_session_normalize")',
    'strcmp(type, "radio_identify")',
    'strcmp(type, "radio_firmware_read")',
    'strcmp(type, "radio_vfo_a_read")',
    'strcmp(type, "radio_vfo_a_set")',
    'strcmp(type, "radio_operating_state_read")',
    'strcmp(type, "radio_mode_read")',
    'strcmp(type, "radio_tx_state_read")',
    '\\"status_seq\\":%llu',
    "rt_radio_execute_typed(&request, &outcome)",
    "if (identity[index] != '0')",
    "published_status_seq = rt_ble_notify_status()",
    "#define MAX_JSON_OBJECT_MEMBERS ((MAX_LOGICAL_BYTES - 2) / 4)",
    'render_error("wrong_session", "none"',
    '\\"code\\":\\"session_exhausted\\"',
    "decoded_keys_equal(json, &object_keys[index]",
    "#define MAX_JSON_DEPTH MAX_LOGICAL_BYTES",
):
    if protocol_boundary not in protocol_source:
        error(f"nRF protocol boundary is missing: {protocol_boundary}")

if "simulator_frequency_hz" in protocol_source:
    error("nRF radio-disconnected image must not manufacture simulated CAT success")
if "object_keys[8][24]" in protocol_source:
    error("strict JSON duplicate detection must not impose a 24-member limit")
if 'observed_tx = "null"' not in ble_source:
    error("nRF status must not infer radio transmit state from sensed PTT")

monotonic_source = (ROOT / "firmware/nrf5340/src/monotonic.c").read_text(
    encoding="utf-8"
)
if (
    "nrf_timer_event_check" not in monotonic_source
    or "wrap_pending" not in monotonic_source
):
    error("monotonic timer read must account for a pending wrap interrupt")

radio_source = (ROOT / "firmware/nrf5340/src/radio_service.c").read_text(
    encoding="utf-8"
)
if 'outcome->error_code = "radio_disconnected"' not in radio_source:
    error("nRF radio adapter must report its explicit disconnected fixture boundary")
radio_code = re.sub(r"/\*.*?\*/|//[^\n]*", "", radio_source, flags=re.DOTALL)
if "raw" in radio_code.lower():
    error("nRF typed radio adapter must expose no raw CAT entry point")

safety_source = (ROOT / "firmware/nrf5340/src/safety_service.c").read_text(
    encoding="utf-8"
)
for reset_boundary in (
    "hwinfo_get_reset_cause(&reset_cause)",
    "reset_cause & RESET_WATCHDOG",
    "initial_release = RT_RELEASE_WATCHDOG",
):
    if reset_boundary not in safety_source:
        error(f"watchdog reset-cause diagnostic boundary is missing: {reset_boundary}")

cache_source = (ROOT / "firmware/nrf5340/src/operation_cache.c").read_text(
    encoding="utf-8"
)
for cache_boundary in (
    "#define CACHE_LIMIT 512",
    "FIXED_PARTITION_ID(operation_cache_partition)",
    "flash_area_write(",
    "crc32_ieee(",
    "rt_response_queue_enqueue(",
    "rt_response_queue_dequeue(",
):
    if cache_boundary not in cache_source:
        error(f"external-QSPI operation cache boundary is missing: {cache_boundary}")

for ble_boundary in (
    "if (offset == 0)",
    "rt_ble_notify_status();",
    "rt_protocol_att_limit_changed();",
    "BT_ATT_ERR_INSUFFICIENT_RESOURCES",
    "command_transfer.accepted = 0",
    "rt_protocol_session_active()",
):
    if ble_boundary not in ble_source:
        error(f"BLE snapshot/session transition boundary is missing: {ble_boundary}")

audio_source = (ROOT / "firmware/nrf5340/src/audio_service.c").read_text(
    encoding="utf-8"
)
if "(int32_t)mono[index] << 8" in audio_source:
    error("nRF playback conversion must not left-shift negative signed PCM")
if "-((-sample + 128) >> 8)" not in audio_source:
    error("nRF capture conversion must round negative samples without a one-LSB bias")

prj_conf = (ROOT / "firmware/nrf5340/prj.conf").read_text(encoding="utf-8")
if "CONFIG_RIGTETHER_PTT_OUTPUT_ENABLED=n" not in prj_conf:
    error("radio-disconnected fixture must default PTT output to disabled")
if "CONFIG_USB_CDC_ACM=n" not in prj_conf:
    error("CDC diagnostics would alter the fixed USB topology")

for path in (ROOT / "firmware").rglob("*"):
    if path.is_file() and path.suffix in {".c", ".h", ".conf"}:
        text = path.read_text(encoding="utf-8").lower()
        if "usb midi" in text and "unimplemented" not in text and "conditional" not in text:
            error(f"{path.relative_to(ROOT)}: USB MIDI must remain conditional and unimplemented")

if ERRORS:
    print("Firmware validation failed:")
    for item in ERRORS:
        print(f"- {item}")
    raise SystemExit(1)

print(
    "Validated exact BLE UUIDs/roles, logical dispatch and response identity, "
    "safety-snapshot status rendering, canonical UAC1 descriptor parity, "
    "radio-disconnected PTT default, and conditional-MIDI boundary."
)
