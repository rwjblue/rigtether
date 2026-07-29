#!/usr/bin/env python3
"""Run RigTether v0 platform-neutral protocol conformance vectors.

This executable model is intentionally independent of Core Bluetooth, AVFAudio,
Swift, and a physical radio. The normative contract is protocol/README.md.
"""

from __future__ import annotations

import argparse
import copy
import json
import struct
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

T_LEASE_MAX = 500
T_CONTINUOUS_MAX = 60_000
T_REARM_MIN = 1_000
FRAME_HEADER = 16
START = 0x01
END = 0x02

REQUIRED_COVERAGE = {
    "normal",
    "framing_runtime_limit",
    "framing_malformed",
    "exact_duplicate",
    "altered_duplicate",
    "stale_replay",
    "out_of_order",
    "malformed_message",
    "wrong_session",
    "stale_boot",
    "post_expiry",
    "session_replacement",
    "reconnect",
    "independent_health",
    "host_route_loss",
    "device_audio_fault",
    "ble_loss",
    "protocol_fault",
    "inhibit",
    "output_failed_to_assert",
    "output_stuck_active",
    "cat_allowlist",
    "cat_denial",
    "radio_fault",
    "continuous_cap",
    "rearm",
    "recovery",
    "first_cause",
    "watchdog",
    "session_negotiation",
    "unsupported_version",
    "missing_capability",
    "limit_negotiation",
    "duplicate_session_start",
    "malformed_command",
    "release_sensing",
    "rearm_continuity",
    "profile_selection",
    "typed_radio_schema",
}

REQUIRED_CAPABILITIES = {
    "ordered_operations",
    "typed_radio_v0",
    "ptt_leases_v0",
    "independent_health_v0",
    "first_cause_fault_v0",
}
DEVICE_ID = "00112233445566778899aabbccddeeff"
DEVICE_FRAME_LIMIT = 185
DEVICE_MAX_MESSAGE_BYTES = 1024
MAX_SESSION_OPERATIONS = 512


class ConformanceError(Exception):
    """A vector or model assertion failed."""


def fail(message: str) -> None:
    raise ConformanceError(message)


def ensure_uint(value: Any, name: str, maximum: int = (1 << 64) - 1) -> int:
    if isinstance(value, bool) or not isinstance(value, int):
        fail(f"{name} must be an integer")
    if value < 0 or value > maximum:
        fail(f"{name} outside 0..{maximum}")
    return value


def is_hex_id(value: Any) -> bool:
    return (
        isinstance(value, str)
        and len(value) == 32
        and all(character in "0123456789abcdef" for character in value)
    )


def canonical_request(event: dict[str, Any], boot_id: str, session_id: str) -> bytes:
    message = {
        "type": "request",
        "v": {"major": 0, "minor": 0},
        "boot_id": event.get("boot_id", boot_id),
        "session_id": event.get("session_id", session_id),
        "op_id": event["op_id"],
        "seq": event["seq"],
        "command": event["command"],
    }
    if "serialization_tag" in event:
        message["_vector_serialization_tag"] = event["serialization_tag"]
    return json.dumps(
        message, ensure_ascii=False, separators=(",", ":"), sort_keys=True
    ).encode("utf-8")


def canonical_session_start(event: dict[str, Any], boot_id: str) -> bytes:
    message = {
        "type": "session_start",
        "boot_id": event.get("boot_id", boot_id),
        "client_nonce": event["client_nonce"],
        "op_id": event["op_id"],
        "select": event["select"],
        "required_capabilities": event["required_capabilities"],
        "client_rx_frame_limit": event["client_rx_frame_limit"],
        "client_max_message_bytes": event["client_max_message_bytes"],
    }
    if "serialization_tag" in event:
        message["_vector_serialization_tag"] = event["serialization_tag"]
    return json.dumps(
        message, ensure_ascii=False, separators=(",", ":"), sort_keys=True
    ).encode("utf-8")


def parse_strict_json(raw: bytes) -> dict[str, Any]:
    def reject_constant(value: str) -> Any:
        fail(f"non-JSON constant {value}")

    def parse_integer(value: str) -> int:
        parsed = int(value)
        if parsed < 0:
            fail("negative integer")
        return parsed

    def pairs(values: list[tuple[str, Any]]) -> dict[str, Any]:
        result: dict[str, Any] = {}
        for key, value in values:
            if key in result:
                fail(f"duplicate key {key}")
            result[key] = value
        return result

    try:
        text = raw.decode("utf-8")
        parsed = json.loads(
            text,
            object_pairs_hook=pairs,
            parse_int=parse_integer,
            parse_float=lambda value: fail(f"float {value}"),
            parse_constant=reject_constant,
        )
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        fail(f"malformed JSON: {exc}")
    if not isinstance(parsed, dict):
        fail("top level is not an object")
    return parsed


def fragment(message: bytes, frame_limit: int, transfer_id: int) -> list[bytes]:
    ensure_uint(frame_limit, "frame_limit", (1 << 32) - 1)
    ensure_uint(transfer_id, "transfer_id", (1 << 32) - 1)
    if frame_limit < 20:
        fail("transport_limit_too_small")
    capacity = frame_limit - FRAME_HEADER
    if not message:
        fail("malformed empty logical message")
    frames = []
    offset = 0
    while offset < len(message):
        chunk = message[offset : offset + capacity]
        flags = (START if offset == 0 else 0) | (
            END if offset + len(chunk) == len(message) else 0
        )
        header = struct.pack(
            ">BBHIII", 0, flags, 0, transfer_id, len(message), offset
        )
        frames.append(header + chunk)
        offset += len(chunk)
    return frames


def reassemble(frames: list[bytes], max_message_bytes: int) -> bytes:
    if not frames:
        fail("missing_start")
    expected_offset = 0
    transfer_id = None
    total_length = None
    data = bytearray()
    ended = False
    for index, frame in enumerate(frames):
        if ended:
            fail("bytes_after_end")
        if len(frame) < FRAME_HEADER:
            fail("short_header")
        version, flags, reserved, current_transfer, current_total, offset = struct.unpack(
            ">BBHIII", frame[:FRAME_HEADER]
        )
        if version != 0:
            fail("unsupported_framing_version")
        if flags & ~0x03:
            fail("unknown_flags")
        if reserved != 0:
            fail("nonzero_reserved")
        if index == 0 and not flags & START:
            fail("missing_start")
        if index > 0 and flags & START:
            fail("unexpected_start")
        if transfer_id is None:
            transfer_id = current_transfer
            total_length = current_total
            if total_length > max_message_bytes:
                fail("message_too_large")
        elif current_transfer != transfer_id:
            fail("interleaved_transfer")
        elif current_total != total_length:
            fail("changed_total_length")
        if offset != expected_offset:
            fail("gap_or_overlap")
        payload = frame[FRAME_HEADER:]
        if expected_offset + len(payload) > total_length:
            fail("payload_past_total")
        data.extend(payload)
        expected_offset += len(payload)
        if flags & END:
            if expected_offset != total_length:
                fail("early_end")
            ended = True
    if not ended:
        fail("missing_end")
    return bytes(data)


@dataclass
class CachedOperation:
    request_bytes: bytes
    response: dict[str, Any]


@dataclass
class Model:
    boot_id: str
    session_id: str | None
    now_ms: int = 0
    next_seq: int = 1
    cache: dict[str, CachedOperation] = field(default_factory=dict)
    seq_to_op: dict[int, str] = field(default_factory=dict)
    ble_link: str = "connected"
    protocol_session: str = "active"
    host_route: str = "unknown"
    device_audio: str = "healthy"
    profile: str = "ready"
    radio_profile: str = "kx2"
    inhibit: str = "closed"
    ptt_out: str = "inactive"
    safety_state: str = "receive_safe"
    commanded: str = "inactive"
    intent_id: str | None = None
    lease_id: str | None = None
    expired_lease_ids: set[str] = field(default_factory=set)
    lease_deadline_ms: int | None = None
    continuous_started_ms: int | None = None
    first_fault_code: str | None = None
    first_fault_id: str | None = None
    first_fault_at_ms: int | None = None
    last_release_code: str | None = None
    last_release_at_ms: int | None = None
    rearm_started_ms: int | None = None
    capped_intent_id: str | None = None
    cap_release_reported: bool = False
    radio_io_count: int = 0
    lease_counter: int = 0
    fault_counter: int = 0
    last_result: dict[str, Any] | None = None
    session_counter: int = 0
    start_client_nonce: str | None = None
    start_op_id: str | None = None
    start_bytes: bytes | None = None
    start_response: dict[str, Any] | None = None

    @classmethod
    def from_initial(cls, initial: dict[str, Any], ids: dict[str, str]) -> "Model":
        session_id = ids["session"] if initial.get("session", True) else None
        model = cls(
            boot_id=ids["boot"],
            session_id=session_id,
            host_route="healthy" if session_id is not None else "unknown",
        )
        for key, value in initial.items():
            if key != "session":
                if not hasattr(model, key):
                    fail(f"unknown initial field {key}")
                setattr(model, key, value)
        if session_id is None:
            model.protocol_session = "none"
        return model

    def new_lease_id(self) -> str:
        self.lease_counter += 1
        return f"{self.lease_counter:032x}"

    def new_fault_id(self) -> str:
        self.fault_counter += 1
        return f"{0xf000 + self.fault_counter:032x}"

    def new_session_id(self) -> str:
        self.session_counter += 1
        return f"{0x4500 + self.session_counter:032x}"

    def receive_safe_inputs(self) -> bool:
        return (
            self.commanded == "inactive"
            and self.ptt_out == "inactive"
            and self.inhibit == "closed"
            and self.host_route == "healthy"
            and self.device_audio == "healthy"
            and self.profile == "ready"
            and self.ble_link == "connected"
            and self.protocol_session in {"active", "faulted"}
        )

    def refresh_rearm(self) -> None:
        if self.first_fault_code != "continuous_cap" or not self.cap_release_reported:
            self.rearm_started_ms = None
        elif self.receive_safe_inputs():
            if self.rearm_started_ms is None:
                self.rearm_started_ms = self.now_ms
        else:
            self.rearm_started_ms = None

    def release(self, code: str, *, close_intent: bool = True) -> None:
        if self.lease_id is not None:
            self.expired_lease_ids.add(self.lease_id)
        self.commanded = "inactive"
        self.lease_id = None
        self.lease_deadline_ms = None
        self.continuous_started_ms = None
        self.last_release_code = code
        self.last_release_at_ms = self.now_ms
        if close_intent:
            self.intent_id = None
        if self.safety_state != "fault_lockout" and self.ptt_out == "inactive":
            self.safety_state = "receive_safe"
        self.refresh_rearm()

    def lockout(self, code: str, *, op_id: str | None = None) -> None:
        self.release(code, close_intent=False)
        self.safety_state = "fault_lockout"
        self.protocol_session = (
            "faulted" if code == "protocol_fault" else self.protocol_session
        )
        if self.first_fault_code is None:
            self.first_fault_code = code
            self.first_fault_id = self.new_fault_id()
            self.first_fault_at_ms = self.now_ms
        self.refresh_rearm()

    def hello(self) -> None:
        self.last_result = {
            "type": "hello",
            "device_id": DEVICE_ID,
            "boot_id": self.boot_id,
            "versions": [{"major": 0, "min_minor": 0, "max_minor": 0}],
            "capabilities": sorted(REQUIRED_CAPABILITIES),
            "profiles": ["kx2", "kx3"],
            "command_frame_limit": DEVICE_FRAME_LIMIT,
            "max_message_bytes": DEVICE_MAX_MESSAGE_BYTES,
            "max_session_operations": MAX_SESSION_OPERATIONS,
        }

    def session_start(self, event: dict[str, Any]) -> None:
        try:
            request_bytes = canonical_session_start(event, self.boot_id)
        except (KeyError, TypeError, ValueError):
            self.last_result = self.error("malformed")
            return
        client_nonce = event.get("client_nonce")
        op_id = event.get("op_id")
        capabilities = event.get("required_capabilities")
        if (
            not is_hex_id(client_nonce)
            or not is_hex_id(op_id)
            or not isinstance(capabilities, list)
            or any(not isinstance(item, str) for item in capabilities)
            or len(capabilities) != len(set(capabilities))
        ):
            self.last_result = self.error("malformed")
            return
        if (
            client_nonce == self.start_client_nonce or op_id == self.start_op_id
        ) and self.start_bytes is not None:
            if request_bytes == self.start_bytes:
                self.last_result = copy.deepcopy(self.start_response)
            else:
                self.last_result = self.error("altered_duplicate")
            return
        if event.get("boot_id", self.boot_id) != self.boot_id:
            self.last_result = self.error("stale_boot")
            return
        if event.get("select") != {"major": 0, "minor": 0}:
            self.last_result = self.error("unsupported_version")
            return
        if not REQUIRED_CAPABILITIES.issuperset(capabilities):
            self.last_result = self.error("missing_capability")
            return
        if not REQUIRED_CAPABILITIES.issubset(capabilities):
            self.last_result = self.error("missing_capability")
            return
        client_limit = event.get("client_rx_frame_limit")
        client_max = event.get("client_max_message_bytes")
        if (
            isinstance(client_limit, bool)
            or not isinstance(client_limit, int)
            or client_limit < 20
        ):
            self.last_result = self.error("transport_limit_too_small")
            return
        if (
            isinstance(client_max, bool)
            or not isinstance(client_max, int)
            or client_max < DEVICE_MAX_MESSAGE_BYTES
        ):
            self.last_result = self.error("transport_limit_too_small")
            return

        self.release("session_replaced")
        self.session_id = self.new_session_id()
        self.next_seq = 1
        self.cache.clear()
        self.seq_to_op.clear()
        self.intent_id = None
        self.protocol_session = "active"
        self.host_route = "unknown"
        selected_limit = min(DEVICE_FRAME_LIMIT, client_limit)
        response = self.ok(
            {
                "type": "session_started",
                "session_id": self.session_id,
                "selected": {"major": 0, "minor": 0},
                "capabilities": sorted(REQUIRED_CAPABILITIES),
                "device_rx_frame_limit": DEVICE_FRAME_LIMIT,
                "device_tx_frame_limit": selected_limit,
                "max_message_bytes": min(DEVICE_MAX_MESSAGE_BYTES, client_max),
                "max_session_operations": MAX_SESSION_OPERATIONS,
                "next_seq": 1,
            }
        )
        self.start_client_nonce = client_nonce
        self.start_op_id = op_id
        self.start_bytes = request_bytes
        self.start_response = copy.deepcopy(response)
        self.last_result = response

    def process_time(self, target_ms: int) -> None:
        if target_ms < self.now_ms:
            fail("vector time moved backward")
        while self.safety_state == "tx_active":
            assert self.lease_deadline_ms is not None
            assert self.continuous_started_ms is not None
            cap_at = self.continuous_started_ms + T_CONTINUOUS_MAX
            next_event = min(self.lease_deadline_ms, cap_at)
            if next_event > target_ms:
                break
            self.now_ms = next_event
            if cap_at <= self.lease_deadline_ms:
                self.capped_intent_id = self.intent_id
                self.lockout("continuous_cap")
            else:
                self.release("lease_expired")
            break
        self.now_ms = target_ms

    def ok(self, result: dict[str, Any]) -> dict[str, Any]:
        return {"ok": True, "result": result}

    def error(
        self, code: str, safety_effect: str = "none", radio_io: bool = False
    ) -> dict[str, Any]:
        return {
            "ok": False,
            "error": {
                "code": code,
                "safety_effect": safety_effect,
                "radio_io_attempted": radio_io,
            },
        }

    def precondition_error(self) -> str | None:
        if self.safety_state == "fault_lockout":
            return "fault_lockout"
        if self.host_route != "healthy":
            return "host_route_unhealthy"
        if self.device_audio != "healthy":
            return "device_audio_unhealthy"
        if self.ble_link != "connected" or self.protocol_session != "active":
            return "control_unhealthy"
        if self.profile != "ready":
            return "profile_not_ready"
        if self.inhibit != "closed":
            return "inhibit_open"
        if self.ptt_out != "inactive":
            return "ptt_out_not_inactive"
        return None

    def execute_command(self, command: dict[str, Any]) -> dict[str, Any]:
        command_type = command.get("type")
        if command_type == "status_read":
            return self.ok({"type": command_type})
        if command_type == "host_audio_route_report":
            health = command.get("health")
            if health not in {"healthy", "unhealthy", "unknown"}:
                return self.error("invalid_argument")
            self.host_route = health
            if health != "healthy" and self.safety_state == "tx_active":
                self.release("host_route_unhealthy")
            self.refresh_rearm()
            return self.ok({"type": command_type, "health": health})
        if command_type == "ptt_intent_begin":
            intent_id = command.get("intent_id")
            if self.safety_state == "fault_lockout":
                return self.error("fault_lockout")
            if self.intent_id is not None:
                return self.error("intent_active")
            if not isinstance(intent_id, str) or len(intent_id) != 32:
                return self.error("invalid_argument")
            self.intent_id = intent_id
            return self.ok({"type": command_type, "intent_id": intent_id})
        if command_type == "ptt_acquire":
            if command.get("intent_id") != self.intent_id or self.intent_id is None:
                return self.error("intent_required")
            if self.lease_id is not None:
                return self.error("lease_active")
            denied = self.precondition_error()
            if denied:
                return self.error(denied)
            requested = command.get("requested_ms")
            if (
                isinstance(requested, bool)
                or not isinstance(requested, int)
                or requested < 1
                or requested > T_LEASE_MAX
            ):
                return self.error("invalid_argument")
            self.lease_id = self.new_lease_id()
            self.lease_deadline_ms = self.now_ms + requested
            self.continuous_started_ms = self.now_ms
            self.commanded = "active"
            self.ptt_out = "active"
            self.safety_state = "tx_active"
            return self.ok(
                {
                    "type": command_type,
                    "lease_id": self.lease_id,
                    "granted_ms": requested,
                    "lease_deadline_ms": self.lease_deadline_ms,
                }
            )
        if command_type == "ptt_renew":
            requested_lease = command.get("lease_id")
            if requested_lease in self.expired_lease_ids:
                return self.error("lease_expired")
            if (
                self.safety_state != "tx_active"
                or requested_lease != self.lease_id
                or command.get("intent_id") != self.intent_id
            ):
                return self.error("lease_not_found")
            if self.now_ms >= (self.lease_deadline_ms or 0):
                self.release("lease_expired")
                return self.error("lease_expired")
            denied = None
            if self.safety_state == "fault_lockout":
                denied = "fault_lockout"
            elif self.host_route != "healthy":
                denied = "host_route_unhealthy"
            elif self.device_audio != "healthy":
                denied = "device_audio_unhealthy"
            elif self.ble_link != "connected" or self.protocol_session != "active":
                denied = "control_unhealthy"
            elif self.profile != "ready":
                denied = "profile_not_ready"
            elif self.inhibit != "closed":
                denied = "inhibit_open"
            if denied:
                self.release(denied)
                return self.error(denied, "release")
            requested = command.get("requested_ms")
            if (
                isinstance(requested, bool)
                or not isinstance(requested, int)
                or requested < 1
                or requested > T_LEASE_MAX
            ):
                return self.error("invalid_argument")
            self.lease_deadline_ms = self.now_ms + requested
            return self.ok(
                {
                    "type": command_type,
                    "lease_id": self.lease_id,
                    "granted_ms": requested,
                    "lease_deadline_ms": self.lease_deadline_ms,
                }
            )
        if command_type == "ptt_release":
            requested_intent = command.get("intent_id")
            if (
                self.first_fault_code == "continuous_cap"
                and requested_intent == self.capped_intent_id
            ):
                self.cap_release_reported = True
                self.intent_id = None
                self.last_release_code = "operator_release"
                self.last_release_at_ms = self.now_ms
                self.refresh_rearm()
                return self.ok({"type": command_type, "released": True})
            if self.intent_id is not None and requested_intent != self.intent_id:
                return self.error("intent_required")
            self.release("operator_release")
            return self.ok({"type": command_type, "released": True})
        if command_type == "safety_recover":
            if self.safety_state != "fault_lockout":
                return self.error("fault_lockout")
            if command.get("fault_id") != self.first_fault_id:
                return self.error("wrong_fault")
            if (
                self.ptt_out != "inactive"
                or self.inhibit != "closed"
                or self.host_route != "healthy"
                or self.device_audio != "healthy"
                or self.profile != "ready"
                or self.ble_link != "connected"
            ):
                return self.error("rearm_incomplete")
            if self.first_fault_code == "continuous_cap":
                if (
                    not self.cap_release_reported
                    or self.rearm_started_ms is None
                    or self.now_ms - self.rearm_started_ms < T_REARM_MIN
                ):
                    return self.error("rearm_incomplete")
            self.safety_state = "receive_safe"
            self.first_fault_code = None
            self.first_fault_id = None
            self.first_fault_at_ms = None
            self.protocol_session = "active" if self.session_id else "none"
            self.capped_intent_id = None
            self.cap_release_reported = False
            self.rearm_started_ms = None
            self.intent_id = None
            return self.ok({"type": command_type, "recovered": True})
        if command_type == "radio_profile_select":
            if set(command) != {"type", "profile"} or command.get("profile") not in {
                "kx2",
                "kx3",
            }:
                return self.error("invalid_argument")
            selected_profile = command["profile"]
            self.release("profile_change")
            self.radio_profile = selected_profile
            self.profile = "validating"
            self.session_id = None
            self.protocol_session = "none"
            self.host_route = "unknown"
            self.next_seq = 1
            self.cache.clear()
            self.seq_to_op.clear()
            return self.ok(
                {
                    "type": command_type,
                    "profile": selected_profile,
                    "state": "validating",
                    "session_invalidated": True,
                }
            )
        radio_results: dict[str, dict[str, Any]] = {
            "radio_session_normalize": {
                "type": "radio_session_normalize",
                "auto_information": "off",
                "k2_mode": "off",
                "k3_extended_mode": "off",
            },
            "radio_identify": {
                "type": "radio_identify",
                "profile": self.radio_profile,
                "product_code": 1 if self.radio_profile == "kx2" else 2,
                "option_flags": ["a", "p", "f", "t", "b", "x", "i"],
            },
            "radio_firmware_read": {
                "type": "radio_firmware_read",
                "main": "03.14",
                "dsp": None,
            },
            "radio_vfo_a_read": {
                "type": "radio_vfo_a_read",
                "frequency_hz": 7_100_000,
            },
            "radio_operating_state_read": {
                "type": "radio_operating_state_read",
                "frequency_hz": 7_100_000,
                "tx_state": "receive",
            },
            "radio_mode_read": {
                "type": "radio_mode_read",
                "mode": "lsb",
            },
            "radio_tx_state_read": {
                "type": "radio_tx_state_read",
                "tx_state": "receive",
            },
        }
        if command_type == "radio_vfo_a_set":
            frequency = command.get("frequency_hz")
            if (
                set(command) != {"type", "frequency_hz"}
                or isinstance(frequency, bool)
                or not isinstance(frequency, int)
                or frequency > 99_999_999_999
            ):
                return self.error("invalid_argument")
            self.radio_io_count += 1
            return self.ok(
                {
                    "type": command_type,
                    "frequency_hz": frequency,
                    "query_verified": True,
                }
            )
        if command_type in radio_results:
            if set(command) != {"type"}:
                return self.error("invalid_argument")
            self.radio_io_count += 1
            return self.ok(radio_results[command_type])
        if command_type.startswith("radio_") or command_type == "raw_cat":
            return self.error("unsupported_radio_operation", radio_io=False)
        return self.error("unsupported_command")

    def request(self, event: dict[str, Any]) -> None:
        if self.session_id is None:
            self.last_result = self.error("wrong_session")
            return
        try:
            request_bytes = canonical_request(event, self.boot_id, self.session_id)
        except (KeyError, TypeError, ValueError):
            self.lockout("protocol_fault")
            self.last_result = self.error("malformed", "lockout")
            return
        op_id = event.get("op_id")
        seq = event.get("seq")
        command = event.get("command")
        if (
            not is_hex_id(op_id)
            or not isinstance(seq, int)
            or isinstance(seq, bool)
            or not isinstance(command, dict)
            or not isinstance(command.get("type"), str)
            or not command["type"]
        ):
            self.lockout("protocol_fault", op_id=op_id)
            self.last_result = self.error("malformed", "lockout")
            return
        if event.get("boot_id", self.boot_id) != self.boot_id:
            self.lockout("protocol_fault", op_id=op_id)
            self.last_result = self.error("stale_boot", "lockout")
            return
        if event.get("session_id", self.session_id) != self.session_id:
            self.lockout("protocol_fault", op_id=op_id)
            self.last_result = self.error("wrong_session", "lockout")
            return
        if op_id in self.cache:
            cached = self.cache[op_id]
            if cached.request_bytes == request_bytes:
                self.last_result = copy.deepcopy(cached.response)
            else:
                self.lockout("protocol_fault", op_id=op_id)
                self.last_result = self.error("altered_duplicate", "lockout")
            return
        if seq in self.seq_to_op:
            self.lockout("protocol_fault", op_id=op_id)
            self.last_result = self.error("stale_operation", "lockout")
            return
        if seq < self.next_seq:
            self.lockout("protocol_fault", op_id=op_id)
            self.last_result = self.error("stale_operation", "lockout")
            return
        if seq > self.next_seq:
            self.lockout("protocol_fault", op_id=op_id)
            self.last_result = self.error("out_of_order", "lockout")
            return
        response = self.execute_command(command)
        if self.session_id is None:
            self.last_result = response
            return
        self.seq_to_op[seq] = op_id
        self.next_seq += 1
        self.cache[op_id] = CachedOperation(request_bytes, copy.deepcopy(response))
        self.last_result = response

    def event(self, event: dict[str, Any], ids: dict[str, str]) -> None:
        action = event["action"]
        if action == "advance":
            self.process_time(self.now_ms + ensure_uint(event["ms"], "advance.ms"))
        elif action == "sustain_to_cap":
            interval = ensure_uint(event["renew_every_ms"], "renew_every_ms")
            requested = ensure_uint(event["requested_ms"], "requested_ms")
            if interval < 1 or requested > T_LEASE_MAX:
                fail("invalid sustain_to_cap interval or duration")
            if self.safety_state != "tx_active":
                fail("sustain_to_cap requires an active lease")
            assert self.continuous_started_ms is not None
            cap_at = self.continuous_started_ms + T_CONTINUOUS_MAX
            renewals = 0
            while self.now_ms + interval < cap_at:
                self.process_time(self.now_ms + interval)
                if self.safety_state != "tx_active":
                    fail("lease expired before continuous cap")
                self.lease_deadline_ms = self.now_ms + requested
                self.next_seq += 1
                renewals += 1
            self.process_time(cap_at)
            self.last_result = {
                "ok": True,
                "result": {
                    "type": "vector_sustain_to_cap",
                    "accepted_renewals": renewals,
                },
            }
        elif action == "request":
            self.request(event)
        elif action == "hello":
            self.hello()
        elif action == "session_start":
            self.session_start(event)
        elif action == "malformed":
            self.lockout("protocol_fault")
            self.last_result = self.error("malformed", "lockout")
        elif action == "set_health":
            domain = event["domain"]
            value = event["value"]
            if domain == "host_route":
                self.host_route = value
                if value != "healthy" and self.safety_state == "tx_active":
                    self.release("host_route_unhealthy")
            elif domain == "device_audio":
                self.device_audio = value
                if value != "healthy" and self.safety_state == "tx_active":
                    self.release("device_audio_unhealthy")
            elif domain == "inhibit":
                self.inhibit = value
                if value == "open" and self.safety_state == "tx_active":
                    self.release("inhibit_open")
            elif domain == "profile":
                self.profile = value
                if value != "ready" and self.safety_state == "tx_active":
                    self.release("profile_fault")
            else:
                fail(f"unknown health domain {domain}")
            self.refresh_rearm()
        elif action == "sense":
            self.ptt_out = event["value"]
            if self.commanded == "active" and self.ptt_out == "inactive":
                self.lockout("output_failed_to_assert")
            elif self.commanded == "inactive" and self.ptt_out == "active":
                self.lockout("output_stuck_active")
            elif (
                self.commanded == "inactive"
                and self.ptt_out == "inactive"
                and self.safety_state != "fault_lockout"
            ):
                self.safety_state = "receive_safe"
            self.refresh_rearm()
        elif action == "fault":
            self.lockout(event["code"])
        elif action == "session_replace":
            self.release("session_replaced")
            self.session_id = event.get("session_id", ids["session2"])
            self.next_seq = 1
            self.cache.clear()
            self.seq_to_op.clear()
            self.intent_id = None
            self.host_route = "unknown"
            self.protocol_session = "active"
        elif action == "disconnect":
            self.release("ble_disconnect")
            self.session_id = None
            self.ble_link = "disconnected"
            self.protocol_session = "none"
            self.host_route = "unknown"
            self.cache.clear()
            self.seq_to_op.clear()
            self.refresh_rearm()
        elif action == "reconnect":
            self.ble_link = "connected"
            self.protocol_session = "none"
            self.session_id = None
            self.host_route = "unknown"
            self.refresh_rearm()
        elif action == "new_session":
            self.session_id = event.get("session_id", ids["session2"])
            self.protocol_session = "active"
            self.host_route = "unknown"
            self.next_seq = 1
            self.cache.clear()
            self.seq_to_op.clear()
            self.refresh_rearm()
        elif action == "profile_validated":
            if self.profile != "validating":
                fail("profile_validated requires validating state")
            self.profile = "ready"
            self.refresh_rearm()
        elif action == "boot":
            self.release("boot_or_update")
            self.boot_id = event.get("boot_id", ids["boot2"])
            self.session_id = None
            self.protocol_session = "none"
            self.next_seq = 1
            self.cache.clear()
            self.seq_to_op.clear()
            self.intent_id = None
            self.first_fault_code = None
            self.first_fault_id = None
            self.host_route = "unknown"
        elif action == "watchdog":
            self.process_time(self.now_ms + 500)
            self.release("watchdog_reset")
            self.boot_id = ids["boot2"]
            self.session_id = None
            self.protocol_session = "none"
            self.host_route = "unknown"
        else:
            fail(f"unknown action {action}")

    def snapshot(self) -> dict[str, Any]:
        elapsed = (
            0
            if self.continuous_started_ms is None
            else self.now_ms - self.continuous_started_ms
        )
        return {
            "now_ms": self.now_ms,
            "boot_id": self.boot_id,
            "session_id": self.session_id,
            "next_seq": self.next_seq,
            "ble_link": self.ble_link,
            "protocol_session": self.protocol_session,
            "host_route": self.host_route,
            "device_audio": self.device_audio,
            "profile": self.profile,
            "radio_profile": self.radio_profile,
            "inhibit": self.inhibit,
            "commanded": self.commanded,
            "ptt_out": self.ptt_out,
            "safety_state": self.safety_state,
            "intent_id": self.intent_id,
            "lease_id": self.lease_id,
            "owner": (
                None
                if self.lease_id is None
                else {
                    "boot_id": self.boot_id,
                    "session_id": self.session_id,
                    "lease_id": self.lease_id,
                    "intent_id": self.intent_id,
                }
            ),
            "lease_deadline_ms": self.lease_deadline_ms,
            "continuous_elapsed_ms": elapsed,
            "first_fault_code": self.first_fault_code,
            "first_fault_id": self.first_fault_id,
            "first_fault_at_ms": self.first_fault_at_ms,
            "last_release_code": self.last_release_code,
            "last_release_at_ms": self.last_release_at_ms,
            "rearm_started_ms": self.rearm_started_ms,
            "radio_io_count": self.radio_io_count,
            "last_result": self.last_result,
        }


def assert_subset(actual: Any, expected: Any, path: str = "$") -> None:
    if isinstance(expected, dict):
        if not isinstance(actual, dict):
            fail(f"{path}: expected object, got {actual!r}")
        for key, value in expected.items():
            if key not in actual:
                fail(f"{path}: missing {key}")
            assert_subset(actual[key], value, f"{path}.{key}")
    elif isinstance(expected, list):
        if actual != expected:
            fail(f"{path}: expected {expected!r}, got {actual!r}")
    elif actual != expected:
        fail(f"{path}: expected {expected!r}, got {actual!r}")


def run_framing_vector(vector: dict[str, Any]) -> None:
    message = vector["message"].encode("utf-8")
    frames = fragment(message, vector["frame_limit"], vector["transfer_id"])
    assert_subset(
        {
            "payload_lengths": [len(item) - FRAME_HEADER for item in frames],
            "flags": [item[1] for item in frames],
            "offsets": [struct.unpack(">I", item[12:16])[0] for item in frames],
            "round_trip": reassemble(frames, vector["max_message_bytes"]).decode(
                "utf-8"
            ),
        },
        vector["expect"],
    )


def run_bad_framing_vector(vector: dict[str, Any]) -> None:
    message = vector["message"].encode("utf-8")
    frames = fragment(message, vector["frame_limit"], vector["transfer_id"])
    mutation = vector["mutation"]
    mutated = list(frames)
    if mutation == "gap":
        frame = bytearray(mutated[1])
        frame[12:16] = struct.pack(">I", struct.unpack(">I", frame[12:16])[0] + 1)
        mutated[1] = bytes(frame)
    elif mutation == "nonzero_reserved":
        frame = bytearray(mutated[0])
        frame[2:4] = b"\x00\x01"
        mutated[0] = bytes(frame)
    elif mutation == "unknown_flags":
        frame = bytearray(mutated[0])
        frame[1] |= 0x80
        mutated[0] = bytes(frame)
    elif mutation == "changed_total_length":
        frame = bytearray(mutated[1])
        frame[8:12] = struct.pack(">I", len(message) + 1)
        mutated[1] = bytes(frame)
    elif mutation == "missing_end":
        frame = bytearray(mutated[-1])
        frame[1] &= ~END
        mutated[-1] = bytes(frame)
    else:
        fail(f"unknown framing mutation {mutation}")
    try:
        reassemble(mutated, vector["max_message_bytes"])
    except ConformanceError as exc:
        if str(exc) != vector["expect_error"]:
            fail(
                f"{vector['id']}: expected {vector['expect_error']}, got {str(exc)}"
            )
    else:
        fail(f"{vector['id']}: malformed frames unexpectedly reassembled")


def run_bad_message_vector(vector: dict[str, Any]) -> None:
    raw = (
        bytes.fromhex(vector["hex"])
        if "hex" in vector
        else vector["message"].encode("utf-8")
    )
    try:
        parse_strict_json(raw)
    except ConformanceError:
        return
    fail(f"{vector['id']}: malformed logical message unexpectedly parsed")


def run_scenario(
    scenario: dict[str, Any], ids: dict[str, str], verbose: bool
) -> None:
    model = Model.from_initial(scenario.get("initial", {}), ids)
    for index, event in enumerate(scenario["steps"], 1):
        model.event(event, ids)
        if "expect" in event:
            try:
                assert_subset(model.snapshot(), event["expect"])
            except ConformanceError as exc:
                fail(f"{scenario['id']} step {index}: {exc}")
        if verbose:
            print(
                f"  {scenario['id']}[{index}] {event['action']}: "
                f"{json.dumps(model.snapshot(), sort_keys=True)}"
            )


def load_vectors(path: Path) -> dict[str, Any]:
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        fail(str(exc))
    if data.get("vector_version") != 1:
        fail("unsupported vector_version")
    return data


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("vectors", type=Path)
    parser.add_argument("--verbose", action="store_true")
    args = parser.parse_args()
    try:
        vectors = load_vectors(args.vectors)
        coverage = set()
        for item in (
            vectors["framing"]
            + vectors["bad_framing"]
            + vectors["bad_messages"]
            + vectors["scenarios"]
        ):
            coverage.update(item.get("covers", []))
        missing = REQUIRED_COVERAGE - coverage
        if missing:
            fail("missing required coverage: " + ", ".join(sorted(missing)))
        for item in vectors["framing"]:
            run_framing_vector(item)
        for item in vectors["bad_framing"]:
            run_bad_framing_vector(item)
        for item in vectors["bad_messages"]:
            run_bad_message_vector(item)
        for scenario in vectors["scenarios"]:
            run_scenario(scenario, vectors["ids"], args.verbose)
    except (ConformanceError, KeyError, TypeError, ValueError) as exc:
        print(f"Conformance failed: {exc}", file=sys.stderr)
        return 1
    total = (
        len(vectors["framing"])
        + len(vectors["bad_framing"])
        + len(vectors["bad_messages"])
        + len(vectors["scenarios"])
    )
    print(
        f"Validated {total} RigTether v0 vectors "
        f"covering {len(REQUIRED_COVERAGE)} required behaviors."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
