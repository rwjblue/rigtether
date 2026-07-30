//! Exact typed-radio adapter backed by the #9 Elecraft CAT core.

use rigtether_elecraft_cat::{
    CatSession, DocumentedFrequencyPolicy, Error, Mode, Operation, OperationResult, OptionFlag,
    Profile, RadioIo, Request, SafetyDirective, TxObservation,
};
use serde_json::{Value, json};

/// Stable radio result or denial reported to the protocol core.
#[derive(Clone, Debug, PartialEq)]
pub struct RadioOutcome {
    /// Whether the typed operation completed.
    pub ok: bool,
    /// Exact result object when successful.
    pub result: Option<Value>,
    /// Stable v0 error code when denied or faulted.
    pub error_code: Option<&'static str>,
    /// Whether the request reached the radio adapter.
    pub radio_io_attempted: bool,
    /// Whether the safety service must release and lock out immediately.
    pub release_and_lockout: bool,
}

impl RadioOutcome {
    fn success(result: Value) -> Self {
        Self {
            ok: true,
            result: Some(result),
            error_code: None,
            radio_io_attempted: true,
            release_and_lockout: false,
        }
    }

    fn error(error: &Error) -> Self {
        Self {
            ok: false,
            result: None,
            error_code: Some(error.code().as_str()),
            radio_io_attempted: error.radio_io_attempted(),
            release_and_lockout: error.safety_directive() == SafetyDirective::ReleaseAndLockout,
        }
    }
}

/// Radio service that exposes no raw-write path.
pub struct TypedRadio<I> {
    cat: CatSession<I>,
}

impl<I: RadioIo> TypedRadio<I> {
    /// Bind the exact CAT core to one selected profile and transport.
    #[must_use]
    pub const fn new(profile: Profile, io: I) -> Self {
        Self {
            cat: CatSession::new(profile, io),
        }
    }

    /// Borrow the typed CAT session for bounded diagnostics.
    #[must_use]
    pub const fn cat(&self) -> &CatSession<I> {
        &self.cat
    }

    /// Execute an exact v0 radio command object.
    ///
    /// Unknown fields, raw CAT, keying-capable names, and unsupported commands are
    /// rejected before `RadioIo` is called.
    #[must_use]
    pub fn execute(&mut self, command: &Value) -> RadioOutcome {
        let Some(object) = command.as_object() else {
            return invalid_argument();
        };
        let Some(command_type) = object.get("type").and_then(Value::as_str) else {
            return invalid_argument();
        };
        let request = match command_type {
            "radio_session_normalize" => Request::Typed(Operation::NormalizeSession),
            "radio_identify" => Request::Typed(Operation::Identify),
            "radio_firmware_read" => Request::Typed(Operation::ReadFirmware { include_dsp: true }),
            "radio_vfo_a_read" => Request::Typed(Operation::ReadFrequency),
            "radio_vfo_a_set" => {
                let Some(frequency_hz) = object.get("frequency_hz").and_then(Value::as_u64) else {
                    return invalid_argument();
                };
                Request::Typed(Operation::SetFrequency { frequency_hz })
            }
            "radio_operating_state_read" => Request::Typed(Operation::ReadOperatingState),
            "radio_mode_read" => Request::Typed(Operation::ReadMode),
            "radio_tx_state_read" => Request::Typed(Operation::ReadTxState),
            "raw_cat" => Request::RawCat(
                object
                    .get("command")
                    .and_then(Value::as_str)
                    .unwrap_or("<malformed>"),
            ),
            name if name.starts_with("radio_") || is_prohibited_radio_operation(name) => {
                Request::Unsupported(name)
            }
            _ => return invalid_argument(),
        };

        match self.cat.execute(request, &DocumentedFrequencyPolicy) {
            Ok(result) => RadioOutcome::success(result_to_json(result, command_type)),
            Err(error) => RadioOutcome::error(&error),
        }
    }
}

fn invalid_argument() -> RadioOutcome {
    RadioOutcome {
        ok: false,
        result: None,
        error_code: Some("invalid_argument"),
        radio_io_attempted: false,
        release_and_lockout: false,
    }
}

fn is_prohibited_radio_operation(name: &str) -> bool {
    matches!(
        name,
        "TX" | "RX"
            | "SWT"
            | "SWH"
            | "KY"
            | "tune"
            | "xmit"
            | "keyer"
            | "power_write"
            | "VOX_write"
            | "mode_write"
            | "menu_write"
            | "baud_write"
    )
}

fn tx_observation(value: TxObservation) -> &'static str {
    match value {
        TxObservation::Receive => "receive",
        TxObservation::TransmitOrPseudoTransmit => "transmit_or_pseudo_transmit",
    }
}

fn mode(value: Mode) -> &'static str {
    match value {
        Mode::Lsb => "lsb",
        Mode::Usb => "usb",
        Mode::Cw => "cw",
        Mode::Fm => "fm",
        Mode::Am => "am",
        Mode::Data => "data",
        Mode::CwReverse => "cw_reverse",
        Mode::DataReverse => "data_reverse",
    }
}

fn option(value: OptionFlag) -> &'static str {
    match value {
        OptionFlag::A => "a",
        OptionFlag::P => "p",
        OptionFlag::F => "f",
        OptionFlag::T => "t",
        OptionFlag::B => "b",
        OptionFlag::X => "x",
        OptionFlag::I => "i",
    }
}

fn result_to_json(result: OperationResult, command_type: &str) -> Value {
    match result {
        OperationResult::SessionNormalized => json!({
            "type": "radio_session_normalize",
            "auto_information": "off",
            "k2_mode": "off",
            "k3_extended_mode": "off"
        }),
        OperationResult::Identity(identity) => json!({
            "type": "radio_identify",
            "profile": identity.profile.as_str(),
            "product_code": identity.product_code,
            "option_flags": identity.option_flags.into_iter().map(option).collect::<Vec<_>>()
        }),
        OperationResult::Firmware(firmware) => json!({
            "type": "radio_firmware_read",
            "main": firmware.main,
            "dsp": firmware.dsp
        }),
        OperationResult::Frequency(frequency_hz) if command_type == "radio_vfo_a_set" => json!({
            "type": command_type,
            "frequency_hz": frequency_hz,
            "query_verified": true
        }),
        OperationResult::Frequency(frequency_hz) => {
            json!({"type": command_type, "frequency_hz": frequency_hz})
        }
        OperationResult::OperatingState(state) => json!({
            "type": "radio_operating_state_read",
            "frequency_hz": state.frequency_hz,
            "tx_state": tx_observation(state.tx_state)
        }),
        OperationResult::Mode(value) => json!({
            "type": "radio_mode_read",
            "mode": mode(value)
        }),
        OperationResult::TxState(value) => json!({
            "type": "radio_tx_state_read",
            "tx_state": tx_observation(value)
        }),
    }
}
