//! Deterministic firmware state model driven by the shared v0 vectors.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value, json};

use crate::{
    MAX_SESSION_OPERATIONS, T_CONTINUOUS_MAX_MS, T_LEASE_MAX_MS, T_REARM_MIN_MS, strict_json,
};

const DEVICE_ID: &str = "00112233445566778899aabbccddeeff";
const DEVICE_FRAME_LIMIT: u64 = 185;
const DEVICE_MAX_MESSAGE_BYTES: u64 = 1024;
const REQUIRED_CAPABILITIES: [&str; 5] = [
    "first_cause_fault_v0",
    "independent_health_v0",
    "ordered_operations",
    "ptt_leases_v0",
    "typed_radio_v0",
];

#[derive(Clone)]
struct CachedOperation {
    request_bytes: Vec<u8>,
    response_bytes: Vec<u8>,
}

/// Fixed identities supplied by the shared vector fixture.
#[derive(Clone, Debug)]
pub struct VectorIds {
    boot: String,
    boot2: String,
    session: String,
    session2: String,
}

impl VectorIds {
    /// Decode the `ids` object from `protocol/vectors/v0.json`.
    ///
    /// # Errors
    ///
    /// Returns a field-specific diagnostic when a required identity is absent.
    pub fn from_value(value: &Value) -> Result<Self, String> {
        let object = value.as_object().ok_or("ids must be an object")?;
        Ok(Self {
            boot: string_field(object, "boot")?.to_owned(),
            boot2: string_field(object, "boot2")?.to_owned(),
            session: string_field(object, "session")?.to_owned(),
            session2: string_field(object, "session2")?.to_owned(),
        })
    }
}

/// Dedicated safety/session simulator state.
///
/// The model keeps commanded PTT, sensed `PTT OUT`, inhibit, CAT/profile state,
/// USB/audio health, BLE health, and protocol-session health as separate values.
#[derive(Clone)]
pub struct Model {
    ids: VectorIds,
    boot_id: String,
    session_id: Option<String>,
    now_ms: u64,
    status_seq: u64,
    next_seq: u64,
    cache: BTreeMap<String, CachedOperation>,
    seq_to_op: BTreeMap<u64, String>,
    ble_link: String,
    protocol_session: String,
    host_route: String,
    device_audio: String,
    profile: String,
    radio_profile: String,
    inhibit: String,
    ptt_out: String,
    safety_state: String,
    commanded: String,
    intent_id: Option<String>,
    lease_id: Option<String>,
    expired_lease_ids: BTreeSet<String>,
    lease_deadline_ms: Option<u64>,
    continuous_started_ms: Option<u64>,
    first_fault_code: Option<String>,
    first_fault_id: Option<String>,
    first_fault_at_ms: Option<u64>,
    last_release_code: Option<String>,
    last_release_at_ms: Option<u64>,
    rearm_started_ms: Option<u64>,
    capped_intent_id: Option<String>,
    cap_release_reported: bool,
    radio_io_count: u64,
    lease_counter: u64,
    fault_counter: u64,
    last_result: Option<Value>,
    session_counter: u64,
    start_client_nonce: Option<String>,
    start_op_id: Option<String>,
    start_bytes: Option<Vec<u8>>,
    start_response: Option<Value>,
    injected_identity_product_code: Option<u64>,
}

impl Model {
    /// Create a receive-safe vector model.
    ///
    /// # Errors
    ///
    /// Rejects unknown initial fields or invalid initial values.
    pub fn from_initial(ids: VectorIds, initial: Option<&Value>) -> Result<Self, String> {
        let has_session = initial
            .and_then(Value::as_object)
            .and_then(|object| object.get("session"))
            .and_then(Value::as_bool)
            .unwrap_or(true);
        let mut model = Self {
            boot_id: ids.boot.clone(),
            session_id: has_session.then(|| ids.session.clone()),
            ids,
            now_ms: 0,
            status_seq: 1,
            next_seq: 1,
            cache: BTreeMap::new(),
            seq_to_op: BTreeMap::new(),
            ble_link: "connected".to_owned(),
            protocol_session: if has_session { "active" } else { "none" }.to_owned(),
            host_route: if has_session { "healthy" } else { "unknown" }.to_owned(),
            device_audio: "healthy".to_owned(),
            profile: "ready".to_owned(),
            radio_profile: "kx2".to_owned(),
            inhibit: "closed".to_owned(),
            ptt_out: "inactive".to_owned(),
            safety_state: "receive_safe".to_owned(),
            commanded: "inactive".to_owned(),
            intent_id: None,
            lease_id: None,
            expired_lease_ids: BTreeSet::new(),
            lease_deadline_ms: None,
            continuous_started_ms: None,
            first_fault_code: None,
            first_fault_id: None,
            first_fault_at_ms: None,
            last_release_code: None,
            last_release_at_ms: None,
            rearm_started_ms: None,
            capped_intent_id: None,
            cap_release_reported: false,
            radio_io_count: 0,
            lease_counter: 0,
            fault_counter: 0,
            last_result: None,
            session_counter: 0,
            start_client_nonce: None,
            start_op_id: None,
            start_bytes: None,
            start_response: None,
            injected_identity_product_code: None,
        };
        if let Some(object) = initial.and_then(Value::as_object) {
            for (key, value) in object {
                match key.as_str() {
                    "session" => {}
                    "host_route" => model.host_route = value_string(value, key)?,
                    "device_audio" => model.device_audio = value_string(value, key)?,
                    "profile" => model.profile = value_string(value, key)?,
                    "radio_profile" => model.radio_profile = value_string(value, key)?,
                    "inhibit" => model.inhibit = value_string(value, key)?,
                    "ptt_out" => model.ptt_out = value_string(value, key)?,
                    "ble_link" => model.ble_link = value_string(value, key)?,
                    "protocol_session" => model.protocol_session = value_string(value, key)?,
                    "safety_state" => model.safety_state = value_string(value, key)?,
                    "first_fault_code" => model.first_fault_code = Some(value_string(value, key)?),
                    "first_fault_id" => model.first_fault_id = Some(value_string(value, key)?),
                    "first_fault_at_ms" => {
                        model.first_fault_at_ms = value.as_u64();
                        if model.first_fault_at_ms.is_none() {
                            return Err(format!("{key} must be an unsigned integer"));
                        }
                    }
                    "cap_release_reported" => {
                        model.cap_release_reported = value
                            .as_bool()
                            .ok_or_else(|| format!("{key} must be a boolean"))?;
                    }
                    "rearm_started_ms" => {
                        model.rearm_started_ms = value.as_u64();
                        if model.rearm_started_ms.is_none() {
                            return Err(format!("{key} must be an unsigned integer"));
                        }
                    }
                    other => return Err(format!("unknown initial field {other}")),
                }
            }
        }
        Ok(model)
    }

    /// Apply one vector action.
    ///
    /// # Errors
    ///
    /// Rejects malformed or unsupported vector actions.
    pub fn event(&mut self, event: &Value) -> Result<(), String> {
        let object = event.as_object().ok_or("event must be an object")?;
        let action = string_field(object, "action")?;
        match action {
            "advance" => {
                let delta = uint_field(object, "ms")?;
                self.process_time(
                    self.now_ms
                        .checked_add(delta)
                        .ok_or("vector time overflow")?,
                )?;
            }
            "sustain_to_cap" => self.sustain_to_cap(object)?,
            "fill_session" => self.fill_session()?,
            "request" => self.request(object)?,
            "hello" => self.hello(),
            "session_start" => self.session_start(object)?,
            "inject_radio_identity" => {
                let product_code = uint_field(object, "product_code")?;
                if !matches!(product_code, 1 | 2) {
                    return Err("injected identity product code must be 1 or 2".to_owned());
                }
                self.injected_identity_product_code = Some(product_code);
            }
            "malformed" => {
                self.lockout("protocol_fault");
                self.last_result = Some(error("malformed", "lockout", false));
            }
            "set_health" => self.set_health(object)?,
            "sense" => self.sense(string_field(object, "value")?),
            "fault" => self.lockout(string_field(object, "code")?),
            "session_replace" => {
                self.release("session_replaced", true);
                self.session_id = Some(
                    object
                        .get("session_id")
                        .and_then(Value::as_str)
                        .unwrap_or(&self.ids.session2)
                        .to_owned(),
                );
                self.next_seq = 1;
                self.cache.clear();
                self.seq_to_op.clear();
                self.intent_id = None;
                self.host_route = "unknown".to_owned();
                self.protocol_session = "active".to_owned();
            }
            "disconnect" => self.disconnect(),
            "reconnect" => {
                self.ble_link = "connected".to_owned();
                self.protocol_session = "none".to_owned();
                self.session_id = None;
                self.host_route = "unknown".to_owned();
                self.refresh_rearm();
            }
            "new_session" => {
                self.session_id = Some(
                    object
                        .get("session_id")
                        .and_then(Value::as_str)
                        .unwrap_or(&self.ids.session2)
                        .to_owned(),
                );
                self.protocol_session = "active".to_owned();
                self.host_route = "unknown".to_owned();
                self.next_seq = 1;
                self.cache.clear();
                self.seq_to_op.clear();
                self.refresh_rearm();
            }
            "profile_validated" => {
                if self.profile != "validating" {
                    return Err("profile_validated requires validating state".to_owned());
                }
                self.profile = "ready".to_owned();
                self.refresh_rearm();
            }
            "boot" => {
                self.release("boot_or_update", true);
                self.boot_id = object
                    .get("boot_id")
                    .and_then(Value::as_str)
                    .unwrap_or(&self.ids.boot2)
                    .to_owned();
                self.session_id = None;
                self.protocol_session = "none".to_owned();
                self.next_seq = 1;
                self.cache.clear();
                self.seq_to_op.clear();
                self.intent_id = None;
                self.first_fault_code = None;
                self.first_fault_id = None;
                self.host_route = "unknown".to_owned();
            }
            "watchdog" => {
                self.process_time(
                    self.now_ms
                        .checked_add(500)
                        .ok_or("watchdog time overflow")?,
                )?;
                self.release("watchdog_reset", true);
                self.boot_id = self.ids.boot2.clone();
                self.session_id = None;
                self.protocol_session = "none".to_owned();
                self.host_route = "unknown".to_owned();
            }
            other => return Err(format!("unknown action {other}")),
        }
        self.status_seq = self.status_seq.saturating_add(1);
        Ok(())
    }

    /// Return the diagnostics subset used by shared vector expectations.
    #[must_use]
    pub fn snapshot(&self) -> Value {
        let elapsed = self
            .continuous_started_ms
            .map_or(0, |start| self.now_ms - start);
        let owner = self.lease_id.as_ref().map(|lease_id| {
            json!({
                "boot_id": self.boot_id,
                "session_id": self.session_id,
                "lease_id": lease_id,
                "intent_id": self.intent_id
            })
        });
        json!({
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
            "owner": owner,
            "lease_deadline_ms": self.lease_deadline_ms,
            "continuous_elapsed_ms": elapsed,
            "first_fault_code": self.first_fault_code,
            "first_fault_id": self.first_fault_id,
            "first_fault_at_ms": self.first_fault_at_ms,
            "last_release_code": self.last_release_code,
            "last_release_at_ms": self.last_release_at_ms,
            "rearm_started_ms": self.rearm_started_ms,
            "cached_operations": self.cache.len(),
            "radio_io_count": self.radio_io_count,
            "last_result": self.last_result
        })
    }

    /// Return one self-contained v0 status snapshot.
    ///
    /// Commanded output and sensed `PTT OUT` remain separate; the latter is never
    /// labeled as RF or a complete radio transmit state.
    #[must_use]
    pub fn protocol_status(&self) -> Value {
        let audio_healthy = self.device_audio == "healthy";
        let owner = self.lease_id.as_ref().map(|lease_id| {
            json!({
                "boot_id": self.boot_id,
                "session_id": self.session_id,
                "lease_id": lease_id,
                "intent_id": self.intent_id
            })
        });
        let first_fault = self.first_fault_code.as_ref().map(|code| {
            json!({
                "fault_id": self.first_fault_id,
                "code": code,
                "at_ms": self.first_fault_at_ms,
                "op_id": null
            })
        });
        let last_release = self.last_release_code.as_ref().map(|code| {
            json!({
                "code": code,
                "at_ms": self.last_release_at_ms
            })
        });
        json!({
            "type": "status",
            "v": {"major": 0, "minor": 0},
            "device_id": DEVICE_ID,
            "boot_id": self.boot_id,
            "session_id": self.session_id,
            "status_seq": self.status_seq,
            "device_time_ms": self.now_ms,
            "health": {
                "ble_link": self.ble_link,
                "protocol_session": self.protocol_session,
                "host_usb_audio_route": self.host_route,
                "device_usb_audio": {
                    "aggregate": self.device_audio,
                    "configured": audio_healthy,
                    "tx_stream": if audio_healthy {"active"} else {"inactive"},
                    "clock": self.device_audio,
                    "buffers": self.device_audio,
                    "converter": self.device_audio
                },
                "radio_profile": self.profile
            },
            "radio": {
                "profile": self.radio_profile,
                "observed_tx": null
            },
            "ptt": {
                "commanded": self.commanded,
                "ptt_out": self.ptt_out,
                "inhibit": self.inhibit,
                "owner": owner,
                "lease_deadline_ms": self.lease_deadline_ms,
                "continuous_started_ms": self.continuous_started_ms,
                "continuous_elapsed_ms": self.continuous_started_ms.map_or(0, |start| self.now_ms - start),
                "safety_state": self.safety_state,
                "last_release": last_release,
                "first_fault": first_fault
            }
        })
    }

    /// Process one exact logical command message.
    ///
    /// Duplicate identity compares the original reassembled UTF-8 bytes, including
    /// whitespace and member order. A malformed current-session message releases and
    /// locks out before returning its stable response object.
    ///
    /// # Errors
    ///
    /// Returns an adapter diagnostic only when the input is not a recognized logical
    /// command envelope. Protocol denials are returned as v0 result objects.
    pub fn handle_logical(&mut self, raw: &[u8]) -> Result<Value, String> {
        if raw.len() > 1024 {
            let active = self.session_id.is_some();
            if active {
                self.lockout("protocol_fault");
            }
            self.last_result = Some(error(
                "message_too_large",
                if active { "lockout" } else { "none" },
                false,
            ));
            return self
                .last_result
                .clone()
                .ok_or_else(|| "message-size denial was not retained".to_owned());
        }
        let object = match strict_json::parse_object(raw) {
            Ok(object) => object,
            Err(parse_error) => {
                let active = self.session_id.is_some();
                if active {
                    self.lockout("protocol_fault");
                }
                self.last_result = Some(error(
                    "malformed",
                    if active { "lockout" } else { "none" },
                    false,
                ));
                return if active {
                    self.last_result
                        .clone()
                        .ok_or_else(|| "malformed denial was not retained".to_owned())
                } else {
                    Err(parse_error.to_string())
                };
            }
        };
        match object.get("type").and_then(Value::as_str) {
            Some("request") => self.request_with_bytes(&object, Some(raw))?,
            Some("session_start") => self.session_start_with_bytes(&object, Some(raw))?,
            unsupported => {
                let active = self.session_id.is_some();
                if active {
                    self.lockout("protocol_fault");
                    self.last_result = Some(error("malformed", "lockout", false));
                } else {
                    return Err(unsupported.map_or_else(
                        || "logical message lacks type".to_owned(),
                        |other| format!("unsupported logical message type {other}"),
                    ));
                }
            }
        }
        self.status_seq = self.status_seq.saturating_add(1);
        self.last_result
            .clone()
            .ok_or_else(|| "logical message produced no response".to_owned())
    }

    fn new_lease_id(&mut self) -> String {
        self.lease_counter += 1;
        format!("{:032x}", self.lease_counter)
    }

    fn new_fault_id(&mut self) -> String {
        self.fault_counter += 1;
        format!("{:032x}", 0xf000 + self.fault_counter)
    }

    fn new_session_id(&mut self) -> String {
        self.session_counter += 1;
        format!("{:032x}", 0x4500 + self.session_counter)
    }

    fn receive_safe_inputs(&self) -> bool {
        self.commanded == "inactive"
            && self.ptt_out == "inactive"
            && self.inhibit == "closed"
            && self.host_route == "healthy"
            && self.device_audio == "healthy"
            && self.profile == "ready"
            && self.ble_link == "connected"
            && self.protocol_session == "active"
    }

    fn refresh_rearm(&mut self) {
        if self.first_fault_code.as_deref() != Some("continuous_cap") || !self.cap_release_reported
        {
            self.rearm_started_ms = None;
        } else if self.receive_safe_inputs() {
            if self.rearm_started_ms.is_none() {
                self.rearm_started_ms = Some(self.now_ms);
            }
        } else {
            self.rearm_started_ms = None;
        }
    }

    fn release(&mut self, code: &str, close_intent: bool) {
        if let Some(lease_id) = self.lease_id.take() {
            self.expired_lease_ids.insert(lease_id);
        }
        self.commanded = "inactive".to_owned();
        self.lease_deadline_ms = None;
        self.continuous_started_ms = None;
        self.last_release_code = Some(code.to_owned());
        self.last_release_at_ms = Some(self.now_ms);
        if close_intent {
            self.intent_id = None;
        }
        if self.safety_state != "fault_lockout" && self.ptt_out == "inactive" {
            self.safety_state = "receive_safe".to_owned();
        }
        self.refresh_rearm();
    }

    fn lockout(&mut self, code: &str) {
        self.release(code, false);
        self.safety_state = "fault_lockout".to_owned();
        if code == "protocol_fault" {
            self.protocol_session = "faulted".to_owned();
        }
        if self.first_fault_code.is_none() {
            self.first_fault_code = Some(code.to_owned());
            self.first_fault_id = Some(self.new_fault_id());
            self.first_fault_at_ms = Some(self.now_ms);
        }
        self.refresh_rearm();
    }

    fn hello(&mut self) {
        self.last_result = Some(json!({
            "type": "hello",
            "device_id": DEVICE_ID,
            "boot_id": self.boot_id,
            "versions": [{"major": 0, "min_minor": 0, "max_minor": 0}],
            "capabilities": REQUIRED_CAPABILITIES,
            "profiles": ["kx2", "kx3"],
            "command_frame_limit": DEVICE_FRAME_LIMIT,
            "max_message_bytes": DEVICE_MAX_MESSAGE_BYTES,
            "max_session_operations": MAX_SESSION_OPERATIONS
        }));
    }

    fn session_start(&mut self, event: &Map<String, Value>) -> Result<(), String> {
        self.session_start_with_bytes(event, None)
    }

    fn deny_malformed_session_start(&mut self) {
        let active = self.session_id.is_some();
        if active {
            self.lockout("protocol_fault");
        }
        self.last_result = Some(error(
            "malformed",
            if active { "lockout" } else { "none" },
            false,
        ));
    }

    fn session_start_with_bytes(
        &mut self,
        event: &Map<String, Value>,
        exact_bytes: Option<&[u8]>,
    ) -> Result<(), String> {
        if exact_bytes.is_some() && event.get("boot_id").and_then(Value::as_str).is_none() {
            self.deny_malformed_session_start();
            return Ok(());
        }
        let request_bytes = if let Some(bytes) = exact_bytes {
            bytes.to_vec()
        } else {
            canonical_session_start(event, &self.boot_id)?
        };
        let Some(client_nonce) = event.get("client_nonce").and_then(Value::as_str) else {
            self.deny_malformed_session_start();
            return Ok(());
        };
        let Some(op_id) = event.get("op_id").and_then(Value::as_str) else {
            self.deny_malformed_session_start();
            return Ok(());
        };
        let Some(capabilities) = event.get("required_capabilities").and_then(Value::as_array)
        else {
            self.deny_malformed_session_start();
            return Ok(());
        };
        if !is_hex_id(client_nonce)
            || !is_hex_id(op_id)
            || capabilities.iter().any(|item| item.as_str().is_none())
        {
            self.deny_malformed_session_start();
            return Ok(());
        }
        let set: BTreeSet<&str> = capabilities.iter().filter_map(Value::as_str).collect();
        if set.len() != capabilities.len() {
            self.deny_malformed_session_start();
            return Ok(());
        }
        if self.start_bytes.is_some()
            && (self.start_client_nonce.as_deref() == Some(client_nonce)
                || self.start_op_id.as_deref() == Some(op_id))
        {
            self.last_result = Some(if self.start_bytes.as_deref() == Some(&request_bytes) {
                self.start_response
                    .clone()
                    .ok_or("missing cached session response")?
            } else {
                error("altered_duplicate", "none", false)
            });
            return Ok(());
        }
        let request_boot_id = event
            .get("boot_id")
            .and_then(Value::as_str)
            .unwrap_or(&self.boot_id);
        if !is_hex_id(request_boot_id) {
            self.deny_malformed_session_start();
            return Ok(());
        }
        if request_boot_id != self.boot_id {
            self.last_result = Some(error("stale_boot", "none", false));
            return Ok(());
        }
        let Some(select) = event.get("select").and_then(Value::as_object) else {
            self.deny_malformed_session_start();
            return Ok(());
        };
        if select.get("major").and_then(Value::as_u64).is_none()
            || select.get("minor").and_then(Value::as_u64).is_none()
        {
            self.deny_malformed_session_start();
            return Ok(());
        }
        if select.get("major").and_then(Value::as_u64) != Some(0)
            || select.get("minor").and_then(Value::as_u64) != Some(0)
        {
            self.last_result = Some(error("unsupported_version", "none", false));
            return Ok(());
        }
        let required: BTreeSet<&str> = REQUIRED_CAPABILITIES.into_iter().collect();
        if set != required {
            self.last_result = Some(error("missing_capability", "none", false));
            return Ok(());
        }
        let (Some(client_limit), Some(client_max)) = (
            event.get("client_rx_frame_limit").and_then(Value::as_u64),
            event
                .get("client_max_message_bytes")
                .and_then(Value::as_u64),
        ) else {
            self.deny_malformed_session_start();
            return Ok(());
        };
        if client_limit < 20 || client_max < DEVICE_MAX_MESSAGE_BYTES {
            self.last_result = Some(error("transport_limit_too_small", "none", false));
            return Ok(());
        }
        self.release("session_replaced", true);
        self.session_id = Some(self.new_session_id());
        self.next_seq = 1;
        self.cache.clear();
        self.seq_to_op.clear();
        self.intent_id = None;
        self.protocol_session = "active".to_owned();
        self.host_route = "unknown".to_owned();
        let response = ok(json!({
            "type": "session_started",
            "session_id": self.session_id,
            "selected": {"major": 0, "minor": 0},
            "capabilities": REQUIRED_CAPABILITIES,
            "device_rx_frame_limit": DEVICE_FRAME_LIMIT,
            "device_tx_frame_limit": DEVICE_FRAME_LIMIT.min(client_limit),
            "max_message_bytes": DEVICE_MAX_MESSAGE_BYTES.min(client_max),
            "max_session_operations": MAX_SESSION_OPERATIONS,
            "next_seq": 1
        }));
        self.start_client_nonce = Some(client_nonce.to_owned());
        self.start_op_id = Some(op_id.to_owned());
        self.start_bytes = Some(request_bytes);
        self.start_response = Some(response.clone());
        self.last_result = Some(response);
        Ok(())
    }

    fn process_time(&mut self, target_ms: u64) -> Result<(), String> {
        if target_ms < self.now_ms {
            return Err("vector time moved backward".to_owned());
        }
        if self.safety_state == "tx_active" {
            let lease = self
                .lease_deadline_ms
                .ok_or("active lease lacks deadline")?;
            let cap = self
                .continuous_started_ms
                .ok_or("active lease lacks continuous timer")?
                + T_CONTINUOUS_MAX_MS;
            let next_event = lease.min(cap);
            if next_event <= target_ms {
                self.now_ms = next_event;
                if cap <= lease {
                    self.capped_intent_id = self.intent_id.clone();
                    self.lockout("continuous_cap");
                } else {
                    self.release("lease_expired", true);
                }
            }
        }
        self.now_ms = target_ms;
        Ok(())
    }

    fn precondition_error(&self) -> Option<&'static str> {
        if self.safety_state == "fault_lockout" {
            Some("fault_lockout")
        } else if self.host_route != "healthy" {
            Some("host_route_unhealthy")
        } else if self.device_audio != "healthy" {
            Some("device_audio_unhealthy")
        } else if self.ble_link != "connected" || self.protocol_session != "active" {
            Some("control_unhealthy")
        } else if self.profile != "ready" {
            Some("profile_not_ready")
        } else if self.inhibit != "closed" {
            Some("inhibit_open")
        } else if self.ptt_out != "inactive" {
            Some("ptt_out_not_inactive")
        } else {
            None
        }
    }

    fn execute_command(&mut self, command: &Map<String, Value>) -> Value {
        let Some(command_type) = command.get("type").and_then(Value::as_str) else {
            return error("invalid_argument", "none", false);
        };
        match command_type {
            "status_read" => ok(json!({
                "type": command_type,
                "status_seq": self.status_seq.saturating_add(1)
            })),
            "host_audio_route_report" => {
                let Some(health) = command.get("health").and_then(Value::as_str) else {
                    return error("invalid_argument", "none", false);
                };
                if !matches!(health, "healthy" | "unhealthy" | "unknown") {
                    return error("invalid_argument", "none", false);
                }
                self.host_route = health.to_owned();
                if health != "healthy" && self.safety_state == "tx_active" {
                    self.release("host_route_unhealthy", true);
                }
                self.refresh_rearm();
                ok(json!({"type": command_type, "health": health}))
            }
            "ptt_intent_begin" => self.intent_begin(command),
            "ptt_acquire" => self.acquire(command),
            "ptt_renew" => self.renew(command),
            "ptt_release" => self.operator_release(command),
            "safety_recover" => self.recover(command),
            "radio_profile_select" => self.profile_select(command),
            "radio_identify" => self.radio_identify(command),
            "radio_vfo_a_set" => self.radio_frequency_set(command),
            "radio_session_normalize"
            | "radio_firmware_read"
            | "radio_vfo_a_read"
            | "radio_operating_state_read"
            | "radio_mode_read"
            | "radio_tx_state_read" => self.radio_read(command_type, command),
            "raw_cat" => error("unsupported_radio_operation", "none", false),
            name if name.starts_with("radio_") || is_keying_capable(name) => {
                error("unsupported_radio_operation", "none", false)
            }
            _ => error("unsupported_command", "none", false),
        }
    }

    fn intent_begin(&mut self, command: &Map<String, Value>) -> Value {
        if self.safety_state == "fault_lockout" {
            return error("fault_lockout", "none", false);
        }
        if self.intent_id.is_some() {
            return error("intent_active", "none", false);
        }
        let Some(intent_id) = command.get("intent_id").and_then(Value::as_str) else {
            return error("invalid_argument", "none", false);
        };
        if !is_hex_id(intent_id) {
            return error("invalid_argument", "none", false);
        }
        self.intent_id = Some(intent_id.to_owned());
        ok(json!({"type": "ptt_intent_begin", "intent_id": intent_id}))
    }

    fn acquire(&mut self, command: &Map<String, Value>) -> Value {
        if command.get("intent_id").and_then(Value::as_str) != self.intent_id.as_deref()
            || self.intent_id.is_none()
        {
            return error("intent_required", "none", false);
        }
        if self.lease_id.is_some() {
            return error("lease_active", "none", false);
        }
        if let Some(denied) = self.precondition_error() {
            return error(denied, "none", false);
        }
        let Some(requested) = command.get("requested_ms").and_then(Value::as_u64) else {
            return error("invalid_argument", "none", false);
        };
        if !(1..=T_LEASE_MAX_MS).contains(&requested) {
            return error("invalid_argument", "none", false);
        }
        self.lease_id = Some(self.new_lease_id());
        self.lease_deadline_ms = Some(self.now_ms + requested);
        self.continuous_started_ms = Some(self.now_ms);
        self.commanded = "active".to_owned();
        self.ptt_out = "active".to_owned();
        self.safety_state = "tx_active".to_owned();
        ok(json!({
            "type": "ptt_acquire",
            "lease_id": self.lease_id,
            "granted_ms": requested,
            "lease_deadline_ms": self.lease_deadline_ms
        }))
    }

    fn renew(&mut self, command: &Map<String, Value>) -> Value {
        let requested_lease = command.get("lease_id").and_then(Value::as_str);
        if requested_lease.is_some_and(|id| self.expired_lease_ids.contains(id)) {
            return error("lease_expired", "none", false);
        }
        if self.safety_state != "tx_active"
            || requested_lease != self.lease_id.as_deref()
            || command.get("intent_id").and_then(Value::as_str) != self.intent_id.as_deref()
        {
            return error("lease_not_found", "none", false);
        }
        if self.now_ms >= self.lease_deadline_ms.unwrap_or(0) {
            self.release("lease_expired", true);
            return error("lease_expired", "none", false);
        }
        let denied = if self.host_route != "healthy" {
            Some("host_route_unhealthy")
        } else if self.device_audio != "healthy" {
            Some("device_audio_unhealthy")
        } else if self.ble_link != "connected" || self.protocol_session != "active" {
            Some("control_unhealthy")
        } else if self.profile != "ready" {
            Some("profile_not_ready")
        } else if self.inhibit != "closed" {
            Some("inhibit_open")
        } else {
            None
        };
        if let Some(code) = denied {
            self.release(code, true);
            return error(code, "release", false);
        }
        let Some(requested) = command.get("requested_ms").and_then(Value::as_u64) else {
            return error("invalid_argument", "none", false);
        };
        if !(1..=T_LEASE_MAX_MS).contains(&requested) {
            return error("invalid_argument", "none", false);
        }
        self.lease_deadline_ms = Some(self.now_ms + requested);
        ok(json!({
            "type": "ptt_renew",
            "lease_id": self.lease_id,
            "granted_ms": requested,
            "lease_deadline_ms": self.lease_deadline_ms
        }))
    }

    fn operator_release(&mut self, command: &Map<String, Value>) -> Value {
        let requested_intent = command.get("intent_id").and_then(Value::as_str);
        if self.first_fault_code.as_deref() == Some("continuous_cap")
            && requested_intent == self.capped_intent_id.as_deref()
        {
            self.cap_release_reported = true;
            self.intent_id = None;
            self.last_release_code = Some("operator_release".to_owned());
            self.last_release_at_ms = Some(self.now_ms);
            self.refresh_rearm();
            return ok(json!({"type": "ptt_release", "released": true}));
        }
        if self.intent_id.is_some() && requested_intent != self.intent_id.as_deref() {
            return error("intent_required", "none", false);
        }
        self.release("operator_release", true);
        ok(json!({"type": "ptt_release", "released": true}))
    }

    fn recover(&mut self, command: &Map<String, Value>) -> Value {
        if self.safety_state != "fault_lockout" {
            return error("fault_lockout", "none", false);
        }
        if command.get("fault_id").and_then(Value::as_str) != self.first_fault_id.as_deref() {
            return error("wrong_fault", "none", false);
        }
        if self.ptt_out != "inactive"
            || self.inhibit != "closed"
            || self.host_route != "healthy"
            || self.device_audio != "healthy"
            || self.profile != "ready"
            || self.ble_link != "connected"
            || self.protocol_session != "active"
        {
            return error("rearm_incomplete", "none", false);
        }
        if self.first_fault_code.as_deref() == Some("continuous_cap")
            && (!self.cap_release_reported
                || self.rearm_started_ms.is_none()
                || self.now_ms - self.rearm_started_ms.unwrap_or(self.now_ms) < T_REARM_MIN_MS)
        {
            return error("rearm_incomplete", "none", false);
        }
        self.safety_state = "receive_safe".to_owned();
        self.first_fault_code = None;
        self.first_fault_id = None;
        self.first_fault_at_ms = None;
        self.protocol_session = if self.session_id.is_some() {
            "active"
        } else {
            "none"
        }
        .to_owned();
        self.capped_intent_id = None;
        self.cap_release_reported = false;
        self.rearm_started_ms = None;
        self.intent_id = None;
        ok(json!({"type": "safety_recover", "recovered": true}))
    }

    fn profile_select(&mut self, command: &Map<String, Value>) -> Value {
        let Some(profile) = command.get("profile").and_then(Value::as_str) else {
            return error("invalid_argument", "none", false);
        };
        if !matches!(profile, "kx2" | "kx3") {
            return error("invalid_argument", "none", false);
        }
        self.release("profile_change", true);
        self.radio_profile = profile.to_owned();
        self.profile = "validating".to_owned();
        self.session_id = None;
        self.protocol_session = "none".to_owned();
        self.host_route = "unknown".to_owned();
        self.next_seq = 1;
        self.cache.clear();
        self.seq_to_op.clear();
        ok(json!({
            "type": "radio_profile_select",
            "profile": profile,
            "state": "validating",
            "session_invalidated": true
        }))
    }

    fn radio_identify(&mut self, _command: &Map<String, Value>) -> Value {
        let expected = if self.radio_profile == "kx2" { 1 } else { 2 };
        let observed = self
            .injected_identity_product_code
            .take()
            .unwrap_or(expected);
        self.radio_io_count += 1;
        if observed != expected {
            self.profile = "faulted".to_owned();
            let transmitting = self.safety_state == "tx_active";
            if transmitting {
                self.lockout("radio_control_fault");
            }
            return error(
                "radio_control_fault",
                if transmitting { "lockout" } else { "none" },
                true,
            );
        }
        ok(json!({
            "type": "radio_identify",
            "profile": self.radio_profile,
            "product_code": observed,
            "option_flags": ["a", "p", "f", "t", "b", "x", "i"]
        }))
    }

    fn radio_frequency_set(&mut self, command: &Map<String, Value>) -> Value {
        let Some(frequency) = command.get("frequency_hz").and_then(Value::as_u64) else {
            return error("invalid_argument", "none", false);
        };
        if frequency > 99_999_999_999 {
            return error("invalid_argument", "none", false);
        }
        self.radio_io_count += 1;
        ok(json!({
            "type": "radio_vfo_a_set",
            "frequency_hz": frequency,
            "query_verified": true
        }))
    }

    fn radio_read(&mut self, command_type: &str, _command: &Map<String, Value>) -> Value {
        self.radio_io_count += 1;
        let result = match command_type {
            "radio_session_normalize" => json!({
                "type": command_type,
                "auto_information": "off",
                "k2_mode": "off",
                "k3_extended_mode": "off"
            }),
            "radio_firmware_read" => {
                json!({"type": command_type, "main": "03.14", "dsp": null})
            }
            "radio_vfo_a_read" => json!({"type": command_type, "frequency_hz": 7_100_000}),
            "radio_operating_state_read" => json!({
                "type": command_type,
                "frequency_hz": 7_100_000,
                "tx_state": "receive"
            }),
            "radio_mode_read" => json!({"type": command_type, "mode": "lsb"}),
            "radio_tx_state_read" => json!({"type": command_type, "tx_state": "receive"}),
            _ => unreachable!("matched radio command"),
        };
        ok(result)
    }

    fn request(&mut self, event: &Map<String, Value>) -> Result<(), String> {
        self.request_with_bytes(event, None)
    }

    fn request_with_bytes(
        &mut self,
        event: &Map<String, Value>,
        exact_bytes: Option<&[u8]>,
    ) -> Result<(), String> {
        if self.session_id.is_none() {
            self.last_result = Some(error("wrong_session", "none", false));
            return Ok(());
        }
        let request_bytes = if let Some(bytes) = exact_bytes {
            bytes.to_vec()
        } else {
            match canonical_request(event, &self.boot_id, self.session_id.as_deref()) {
                Ok(bytes) => bytes,
                Err(_) => {
                    self.lockout("protocol_fault");
                    self.last_result = Some(error("malformed", "lockout", false));
                    return Ok(());
                }
            }
        };
        let op_id = event.get("op_id").and_then(Value::as_str);
        let seq = event.get("seq").and_then(Value::as_u64);
        let command = event.get("command").and_then(Value::as_object);
        let request_boot_id = event
            .get("boot_id")
            .and_then(Value::as_str)
            .map(str::to_owned);
        let request_session_id = event
            .get("session_id")
            .and_then(Value::as_str)
            .map(str::to_owned);
        if (exact_bytes.is_some() && (request_boot_id.is_none() || request_session_id.is_none()))
            || (exact_bytes.is_some() && event.get("v") != Some(&json!({"major": 0, "minor": 0})))
            || op_id.is_none_or(|id| !is_hex_id(id))
            || seq.is_none()
            || command
                .and_then(|value| value.get("type"))
                .and_then(Value::as_str)
                .is_none_or(str::is_empty)
        {
            self.lockout("protocol_fault");
            self.last_result = Some(error("malformed", "lockout", false));
            return Ok(());
        }
        let op_id = op_id.expect("validated");
        let seq = seq.expect("validated");
        let request_boot_id = request_boot_id.unwrap_or_else(|| self.boot_id.clone());
        let request_session_id = request_session_id.or_else(|| self.session_id.clone());
        if self.session_id.is_none() {
            self.last_result = Some(error("wrong_session", "none", false));
            return Ok(());
        }
        if request_boot_id != self.boot_id {
            self.lockout("protocol_fault");
            self.last_result = Some(error("stale_boot", "lockout", false));
            return Ok(());
        }
        if request_session_id.as_deref() != self.session_id.as_deref() {
            self.lockout("protocol_fault");
            self.last_result = Some(error("wrong_session", "lockout", false));
            return Ok(());
        }
        if let Some(cached) = self.cache.get(op_id) {
            self.last_result = Some(if cached.request_bytes == request_bytes {
                serde_json::from_slice(&cached.response_bytes)
                    .map_err(|error| format!("cached response corrupted: {error}"))?
            } else {
                self.lockout("protocol_fault");
                error("altered_duplicate", "lockout", false)
            });
            return Ok(());
        }
        if self.seq_to_op.contains_key(&seq) || seq < self.next_seq {
            self.lockout("protocol_fault");
            self.last_result = Some(error("stale_operation", "lockout", false));
            return Ok(());
        }
        if seq > self.next_seq {
            self.lockout("protocol_fault");
            self.last_result = Some(error("out_of_order", "lockout", false));
            return Ok(());
        }
        if self.next_seq > MAX_SESSION_OPERATIONS {
            self.last_result = Some(response_envelope(
                &request_boot_id,
                request_session_id.as_deref().expect("active session"),
                op_id,
                seq,
                self.now_ms,
                self.next_seq,
                error("session_exhausted", "none", false),
            ));
            return Ok(());
        }
        let accepted_at_ms = self.now_ms;
        let response_body = self.execute_command(command.expect("validated"));
        if self.session_id.is_none() {
            self.last_result = Some(response_envelope(
                &request_boot_id,
                request_session_id.as_deref().expect("active session"),
                op_id,
                seq,
                accepted_at_ms,
                self.next_seq,
                response_body,
            ));
            return Ok(());
        }
        self.seq_to_op.insert(seq, op_id.to_owned());
        self.next_seq += 1;
        let response = response_envelope(
            &request_boot_id,
            request_session_id.as_deref().expect("active session"),
            op_id,
            seq,
            accepted_at_ms,
            self.next_seq,
            response_body,
        );
        self.cache.insert(
            op_id.to_owned(),
            CachedOperation {
                request_bytes,
                response_bytes: serde_json::to_vec(&response)
                    .map_err(|error| format!("response serialization failed: {error}"))?,
            },
        );
        self.last_result = Some(response);
        Ok(())
    }

    fn sustain_to_cap(&mut self, event: &Map<String, Value>) -> Result<(), String> {
        let interval = uint_field(event, "renew_every_ms")?;
        let requested = uint_field(event, "requested_ms")?;
        if interval < 1 || requested > T_LEASE_MAX_MS || self.safety_state != "tx_active" {
            return Err("invalid sustain_to_cap request".to_owned());
        }
        let cap_at = self
            .continuous_started_ms
            .ok_or("sustain_to_cap requires continuous timer")?
            + T_CONTINUOUS_MAX_MS;
        let mut renewals = 0;
        while self.now_ms + interval < cap_at {
            self.process_time(self.now_ms + interval)?;
            if self.safety_state != "tx_active" {
                return Err("lease expired before continuous cap".to_owned());
            }
            let event = json!({
                "op_id": format!("{:032x}", 0x7000 + self.next_seq),
                "seq": self.next_seq,
                "command": {
                    "type": "ptt_renew",
                    "intent_id": self.intent_id,
                    "lease_id": self.lease_id,
                    "requested_ms": requested
                }
            });
            self.request(event.as_object().expect("object"))?;
            if self
                .last_result
                .as_ref()
                .and_then(|result| result.get("ok"))
                .and_then(Value::as_bool)
                != Some(true)
            {
                return Err("sustain_to_cap renewal failed".to_owned());
            }
            renewals += 1;
        }
        self.process_time(cap_at)?;
        self.last_result = Some(ok(json!({
            "type": "vector_sustain_to_cap",
            "accepted_renewals": renewals
        })));
        Ok(())
    }

    fn fill_session(&mut self) -> Result<(), String> {
        while self.next_seq <= MAX_SESSION_OPERATIONS {
            let seq = self.next_seq;
            let event = json!({
                "op_id": format!("{:032x}", 0x5000 + seq),
                "seq": seq,
                "command": {"type": "status_read"}
            });
            self.request(event.as_object().expect("object"))?;
            if self
                .last_result
                .as_ref()
                .and_then(|result| result.get("ok"))
                .and_then(Value::as_bool)
                != Some(true)
            {
                return Err("fill_session operation failed".to_owned());
            }
        }
        Ok(())
    }

    fn set_health(&mut self, event: &Map<String, Value>) -> Result<(), String> {
        let domain = string_field(event, "domain")?;
        let value = string_field(event, "value")?.to_owned();
        match domain {
            "host_route" => {
                self.host_route = value;
                if self.host_route != "healthy" && self.safety_state == "tx_active" {
                    self.release("host_route_unhealthy", true);
                }
            }
            "device_audio" => {
                self.device_audio = value;
                if self.device_audio != "healthy" && self.safety_state == "tx_active" {
                    self.release("device_audio_unhealthy", true);
                }
            }
            "inhibit" => {
                self.inhibit = value;
                if self.inhibit == "open" && self.safety_state == "tx_active" {
                    self.release("inhibit_open", true);
                }
            }
            "profile" => {
                self.profile = value;
                if self.profile != "ready" && self.safety_state == "tx_active" {
                    self.release("profile_fault", true);
                }
            }
            other => return Err(format!("unknown health domain {other}")),
        }
        self.refresh_rearm();
        Ok(())
    }

    fn sense(&mut self, value: &str) {
        self.ptt_out = value.to_owned();
        if self.commanded == "active" && self.ptt_out == "inactive" {
            self.lockout("output_failed_to_assert");
        } else if self.commanded == "inactive" && self.ptt_out == "active" {
            self.lockout("output_stuck_active");
        } else if self.commanded == "inactive"
            && self.ptt_out == "inactive"
            && self.safety_state != "fault_lockout"
        {
            self.safety_state = "receive_safe".to_owned();
        }
        self.refresh_rearm();
    }

    fn disconnect(&mut self) {
        self.release("ble_disconnect", true);
        self.session_id = None;
        self.ble_link = "disconnected".to_owned();
        self.protocol_session = "none".to_owned();
        self.host_route = "unknown".to_owned();
        self.cache.clear();
        self.seq_to_op.clear();
        self.start_client_nonce = None;
        self.start_op_id = None;
        self.start_bytes = None;
        self.start_response = None;
        self.refresh_rearm();
    }
}

fn canonical_request(
    event: &Map<String, Value>,
    boot_id: &str,
    session_id: Option<&str>,
) -> Result<Vec<u8>, String> {
    let mut message = Map::new();
    message.insert("type".to_owned(), Value::String("request".to_owned()));
    message.insert("v".to_owned(), json!({"major": 0, "minor": 0}));
    message.insert(
        "boot_id".to_owned(),
        event
            .get("boot_id")
            .cloned()
            .unwrap_or_else(|| Value::String(boot_id.to_owned())),
    );
    message.insert(
        "session_id".to_owned(),
        event
            .get("session_id")
            .cloned()
            .unwrap_or_else(|| session_id.map_or(Value::Null, |id| Value::String(id.to_owned()))),
    );
    for key in ["op_id", "seq", "command"] {
        message.insert(
            key.to_owned(),
            event
                .get(key)
                .cloned()
                .ok_or_else(|| format!("missing {key}"))?,
        );
    }
    if let Some(tag) = event.get("serialization_tag") {
        message.insert("_vector_serialization_tag".to_owned(), tag.clone());
    }
    serde_json::to_vec(&Value::Object(message)).map_err(|error| error.to_string())
}

fn canonical_session_start(event: &Map<String, Value>, boot_id: &str) -> Result<Vec<u8>, String> {
    let keys = [
        "client_nonce",
        "op_id",
        "select",
        "required_capabilities",
        "client_rx_frame_limit",
        "client_max_message_bytes",
    ];
    let mut message = Map::new();
    message.insert("type".to_owned(), Value::String("session_start".to_owned()));
    message.insert(
        "boot_id".to_owned(),
        event
            .get("boot_id")
            .cloned()
            .unwrap_or_else(|| Value::String(boot_id.to_owned())),
    );
    for key in keys {
        message.insert(
            key.to_owned(),
            event
                .get(key)
                .cloned()
                .ok_or_else(|| format!("missing {key}"))?,
        );
    }
    if let Some(tag) = event.get("serialization_tag") {
        message.insert("_vector_serialization_tag".to_owned(), tag.clone());
    }
    serde_json::to_vec(&Value::Object(message)).map_err(|error| error.to_string())
}

fn ok(result: Value) -> Value {
    json!({"ok": true, "result": result})
}

fn error(code: &str, safety_effect: &str, radio_io_attempted: bool) -> Value {
    json!({
        "ok": false,
        "error": {
            "code": code,
            "safety_effect": safety_effect,
            "radio_io_attempted": radio_io_attempted
        }
    })
}

fn response_envelope(
    boot_id: &str,
    session_id: &str,
    op_id: &str,
    seq: u64,
    accepted_at_ms: u64,
    next_seq: u64,
    body: Value,
) -> Value {
    let mut response = Map::new();
    response.insert("type".to_owned(), Value::String("response".to_owned()));
    response.insert("v".to_owned(), json!({"major": 0, "minor": 0}));
    response.insert("boot_id".to_owned(), Value::String(boot_id.to_owned()));
    response.insert(
        "session_id".to_owned(),
        Value::String(session_id.to_owned()),
    );
    response.insert("op_id".to_owned(), Value::String(op_id.to_owned()));
    response.insert("seq".to_owned(), Value::from(seq));
    response.insert("accepted_at_ms".to_owned(), Value::from(accepted_at_ms));
    response.insert("next_seq".to_owned(), Value::from(next_seq));
    if let Some(object) = body.as_object() {
        if let Some(ok) = object.get("ok") {
            response.insert("ok".to_owned(), ok.clone());
        }
        if let Some(result) = object.get("result") {
            response.insert("result".to_owned(), result.clone());
        }
        if let Some(error) = object.get("error") {
            response.insert("error".to_owned(), error.clone());
        }
    }
    Value::Object(response)
}

fn is_hex_id(value: &str) -> bool {
    value.len() == 32
        && value
            .bytes()
            .all(|character| character.is_ascii_digit() || (b'a'..=b'f').contains(&character))
}

fn is_keying_capable(value: &str) -> bool {
    matches!(
        value,
        "TX" | "RX" | "SWT" | "SWH" | "KY" | "tune" | "xmit" | "keyer"
    )
}

fn string_field<'a>(object: &'a Map<String, Value>, key: &str) -> Result<&'a str, String> {
    object
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{key} must be a string"))
}

fn uint_field(object: &Map<String, Value>, key: &str) -> Result<u64, String> {
    object
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("{key} must be an unsigned integer"))
}

fn value_string(value: &Value, key: &str) -> Result<String, String> {
    value
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| format!("{key} must be a string"))
}
