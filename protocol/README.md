# RigTether v0 control protocol

This document is the normative RigTether v0 control contract. It defines logical
messages independently of host APIs, then defines how BLE GATT carries those messages.
The same logical messages and conformance vectors are available to a future Android
client and to the conditional USB MIDI transport adapter without importing Apple
framework behavior.

The contract refines, and does not replace:

- [ADR 0003](../docs/decisions/0003-usb-audio-plus-ble-control.md): USB Audio Class
  carries bidirectional media, BLE GATT carries control, and their health is
  independent;
- [ADR 0004](../docs/decisions/0004-bound-transmit-authority-with-device-enforced-leases.md):
  firmware alone owns one bounded, normally open hardware-PTT path; and
- [ADR 0005](../docs/decisions/0005-separate-host-media-control-radio-and-safety-boundaries.md):
  platform APIs stop at host adapters, the boot coordinator creates `boot_id`, and
  the dedicated safety service owns PTT state and clocks.

The terms **MUST**, **MUST NOT**, **SHOULD**, and **MAY** are normative.

## Version and compatibility

The first version is protocol major `0`, minor `0`, written `0.0`.

- A major version changes existing meaning or required fields. Peers MUST select the
  same major.
- A minor version may add optional fields, enum values, message types, or
  capabilities. A receiver MUST ignore an unknown field unless the selected
  capability declares it required. It MUST reject an unknown command, required
  capability, or enum value with a stable error rather than guessing.
- `hello` advertises every supported `(major, min_minor, max_minor)` range. A v0.0
  host selects `0.0`; no session exists when there is no common range.
- A capability name is part of compatibility. A host lists the capabilities it
  requires in `session_start`; the device atomically denies the session if any are
  absent.

The required v0.0 capabilities are:

```text
ordered_operations
typed_radio_v0
ptt_leases_v0
independent_health_v0
first_cause_fault_v0
```

Backward compatibility before v0.0 is intentionally nonexistent. A later pre-1.0
major may be incompatible.

## Encoding and scalar types

A logical message is one RFC 8259 JSON object encoded as UTF-8.

- JSON numbers MUST be non-negative integers in the range documented for the field.
  Floating point, exponent notation, negative zero, `NaN`, and infinity are invalid.
- Duplicate object keys, invalid UTF-8, trailing bytes, and a top-level value other
  than one object are malformed.
- UUID and opaque 128-bit identity fields use 32 lowercase hexadecimal characters
  without separators.
- Duration and device-time fields are integer milliseconds.
- Hosts MUST NOT attach authority to JSON member order or whitespace. For operation
  duplicate detection, however, “exact” means the complete reassembled UTF-8 bytes
  are byte-for-byte identical. A different serialization reusing an operation
  identity is an altered duplicate.
- A message MUST be no larger than the negotiated `max_message_bytes`. Both peers
  MUST support at least 1024 bytes in v0.0.

The wire contains no wall-clock timestamp. `device_time_ms` is the firmware monotonic
clock since the current boot. It may wrap only after `2^64 - 1` and never moves
backward during one `boot_id`.

## BLE GATT identity and roles

RigTether advertises the following 128-bit primary service UUID:

```text
3dbbf179-78cb-4753-9c4e-092e2a4e1116
```

The service has four characteristics:

| Role | UUID | Properties | Contract |
| --- | --- | --- | --- |
| `hello` | `686fd375-482c-407b-856d-3a5e757f9f03` | read | One unframed `hello` message. It MUST fit in one current read value. Reading does not create a session. |
| `command` | `f98fec49-c274-40d9-9b4b-38f5cef50e15` | write with response | Host-to-device framed `session_start` and request messages. The ATT write response acknowledges only fragment acceptance. |
| `response` | `7ba61ae1-e6b0-4e3f-a718-fd31315cd876` | indicate | Device-to-host framed operation responses and connection-scoped errors. Indication acknowledgement is delivery flow control, never safety authority. |
| `status` | `f9ecbae1-3204-4870-b338-8f220e000585` | read, notify | Latest complete status snapshot. Notifications are framed and may be dropped; a read returns the latest snapshot for resynchronization. |

Service discovery or a BLE connection does not create a protocol session and never
restores transmit intent. The `hello` read is descriptive, not authenticated device
identity.

Commands receive one logical response when the request identity can be parsed. A
client MUST use the response indication as the operation result; an ATT write response
does not mean that an operation was accepted. Missing response or indication
acknowledgement cannot extend a PTT deadline.

Status is a self-contained snapshot, not a delta. `status_seq` increments for every
published snapshot. A gap causes the host to read the characteristic; it does not
change device safety state.

## GATT fragment envelope and runtime sizing

`hello` is unframed. Every value on `command`, `response`, or notified `status` starts
with this 16-byte, network-byte-order envelope:

| Offset | Width | Field |
| --- | --- | --- |
| 0 | 1 | framing version, exactly `0` |
| 1 | 1 | flags: bit 0 `START`, bit 1 `END`; bits 2–7 MUST be zero |
| 2 | 2 | reserved, exactly zero |
| 4 | 4 | direction-local `transfer_id` |
| 8 | 4 | total logical-message byte length |
| 12 | 4 | zero-based byte offset of this fragment |
| 16 | remaining | logical-message bytes |

For each direction:

1. The adapter obtains the current maximum characteristic-value length from its BLE
   stack at runtime. It MUST NOT encode a fixed Apple MTU.
2. `hello.command_frame_limit` reports the device adapter's current maximum command
   value. The host fragments `session_start` to the smaller of that value and its
   current write-with-response limit. `session_start` reports the host receive value
   limit. The device response reports the selected host-to-device and device-to-host
   value limits. Each selected limit is the minimum of the peers' declared/observed
   limits for that direction. A later limit change requires a fresh receive-safe
   session.
3. A selected value limit MUST be at least 20 bytes, leaving at least four payload
   bytes after the envelope. A smaller limit is `transport_limit_too_small`.
4. Every fragment value is no larger than the selected direction limit. The payload
   capacity is `selected_limit - 16`.
5. Fragments for one transfer are contiguous and ordered. The first has `START` and
   offset zero. Each later offset equals the number of bytes already accepted. The
   final fragment has `END`, and its end offset equals `total_length`.
6. `transfer_id` MUST NOT be reused in one direction while its earlier transfer is
   incomplete. It wraps modulo `2^32` only after completion.
7. A peer MUST reject an unknown flag, nonzero reserved field, changed total length,
   gap, overlap, interleaved reuse, length beyond `max_message_bytes`, missing
   `START`, early `END`, or bytes after `END`.

The BLE adapter may discard an incomplete transfer after an implementation-defined
timeout. Timeout and fragment failure are transport faults, not logical operation
results. If a session is active, a malformed command transfer is a protocol fault:
the safety service releases an active lease and enters `FAULT_LOCKOUT`. An incomplete
status notification is discarded and recovered by reading the latest snapshot.

The adapter serializes response indications so that a later operation response cannot
overtake an earlier one. This ordering is transport delivery behavior; operation
sequencing remains enforced by the logical envelope below.

## Identity and session establishment

### Identity fields

| Field | Creator | Lifetime and meaning |
| --- | --- | --- |
| `device_id` | manufacturing/provisioning boundary | Stable opaque 128-bit product identity. It is not a credential. |
| `boot_id` | boot/update coordinator only | Fresh opaque 128-bit value created once before services start and immutable until the next firmware boot. |
| `client_nonce` | host | Fresh opaque 128-bit value for one session-start attempt. It carries no transmit intent. |
| `session_id` | protocol/session core | Fresh opaque 128-bit value for one accepted session. |
| `op_id` | host | Fresh opaque 128-bit identity for one logical operation in a session. |
| `intent_id` | host | Fresh opaque 128-bit identity for one explicit operator-intent epoch. |
| `lease_id` | PTT safety service | Unpredictable or monotonically unique 128-bit identity within the session. |
| `fault_id` | PTT safety service | Fresh 128-bit identity for one latched first-cause fault. |

Neither reconnect nor host state restoration may reuse `client_nonce`, `session_id`,
`intent_id`, or a lease. A reboot changes `boot_id` before a new session can exist.

### `hello`

The read-only message has:

```json
{
  "type": "hello",
  "device_id": "00112233445566778899aabbccddeeff",
  "boot_id": "11111111111111111111111111111111",
  "versions": [{"major": 0, "min_minor": 0, "max_minor": 0}],
  "capabilities": ["first_cause_fault_v0", "independent_health_v0",
    "ordered_operations", "ptt_leases_v0", "typed_radio_v0"],
  "profiles": ["kx2", "kx3"],
  "command_frame_limit": 185,
  "max_message_bytes": 1024,
  "max_session_operations": 512
}
```

Capability and profile arrays are sets; ordering has no meaning. A device MUST NOT
advertise a profile whose typed surface violates the allowlist below.

### `session_start`

`session_start` is the only command that has no `session_id` or sequence:

```json
{
  "type": "session_start",
  "boot_id": "11111111111111111111111111111111",
  "client_nonce": "22222222222222222222222222222222",
  "op_id": "33333333333333333333333333333333",
  "select": {"major": 0, "minor": 0},
  "required_capabilities": ["ordered_operations", "typed_radio_v0",
    "ptt_leases_v0", "independent_health_v0", "first_cause_fault_v0"],
  "client_rx_frame_limit": 185,
  "client_max_message_bytes": 4096
}
```

Acceptance atomically performs these steps:

1. request the safety service to make PTT inactive and invalidate any lease;
2. invalidate the old protocol session and its operations;
3. create a fresh `session_id`;
4. reset the new session's expected sequence to `1`; and
5. reset `host_usb_audio_route` to `unknown`; and
6. publish a receive-safe status before any PTT acquire can succeed.

The response contains `session_id`, selected version/capabilities,
`device_rx_frame_limit`, `device_tx_frame_limit`, negotiated
`max_message_bytes`, `max_session_operations`, and `next_seq: 1`.

An exact duplicate `session_start` with the same current `client_nonce`, `op_id`, and
bytes returns the cached start response without replacing the session. Altered reuse
of either identity is `altered_duplicate`. A different fresh start always replaces
the current session receive-safely. A stale `boot_id` is `stale_boot` and creates no
session.

BLE reconnect, a restored peripheral object, or a second central must use a new
`session_start`; none inherits the prior session. A detected BLE disconnect
immediately invalidates the session and requests release. Silent loss is still bounded
by the current device lease deadline. Because host-route health is a session-scoped
claim, every accepted start requires the new central to send a fresh
`host_audio_route_report` before `ptt_acquire` can succeed.

## Ordered operations and acknowledgement

Every ordinary request uses:

```json
{
  "type": "request",
  "v": {"major": 0, "minor": 0},
  "boot_id": "11111111111111111111111111111111",
  "session_id": "44444444444444444444444444444444",
  "op_id": "55555555555555555555555555555555",
  "seq": 1,
  "command": {"type": "status_read"}
}
```

Every response echoes `boot_id`, `session_id`, `op_id`, and `seq`, and contains:

```json
{
  "type": "response",
  "v": {"major": 0, "minor": 0},
  "boot_id": "11111111111111111111111111111111",
  "session_id": "44444444444444444444444444444444",
  "op_id": "55555555555555555555555555555555",
  "seq": 1,
  "accepted_at_ms": 1200,
  "next_seq": 2,
  "ok": true,
  "result": {"type": "status_read", "status_seq": 7}
}
```

For a denied operation, `ok` is false and `error` contains:

```json
{
  "code": "inhibit_open",
  "safety_effect": "none",
  "radio_io_attempted": false
}
```

The rules are exact:

1. A new request MUST use the current `boot_id`, current `session_id`, a never-used
   `op_id`, and exactly `seq == next_seq`.
2. A syntactically and semantically valid new request consumes its sequence even when
   the requested operation is denied. Its response reports the next sequence.
3. The device retains sufficient request identity material to prove byte-exact
   equality, plus the exact serialized response, for every consumed operation until
   the session ends. It MUST NOT evict entries or treat a digest collision as an exact
   duplicate. Both peers negotiate at least 512 operations per session. When the limit
   is exhausted, further new operations return `session_exhausted`; the host replaces
   the session receive-safely. Exhaustion is checked only for a fresh operation at
   `next_seq`; an exact duplicate of any retained operation still returns its cached
   response and does not consume another slot.
4. A byte-exact request with the same `op_id` and `seq` returns the cached response
   bytes. It consumes no sequence, repeats no radio I/O or transition, and changes no
   lease deadline.
5. Reuse of an `op_id` with different bytes, including a different sequence, is
   `altered_duplicate`.
6. A never-seen `op_id` below `next_seq` is `stale_operation`, even when its sequence
   was previously consumed by another operation. A sequence above `next_seq` is
   `out_of_order`.
7. A parseable request with stale `boot_id`, wrong `session_id`, altered duplicate,
   stale operation, or out-of-order sequence is a current-channel protocol fault. It
   cannot reach radio I/O, releases an active lease, and latches `FAULT_LOCKOUT`.
8. Malformed JSON, framing, or required fields on `command` has the same lockout
   effect when a session is active. Before a session, it is simply rejected.
9. A response is descriptive. Host receipt, loss, retry, or acknowledgement never
   extends authority.

Stable `safety_effect` values are `none`, `release`, `lockout`, and
`session_replace`. Stable error codes are listed with commands and vectors; an
implementation may attach an optional human-readable `detail`, but clients MUST NOT
branch on it.

## Typed radio surface

There is no raw CAT operation in v0.0. The protocol/session core recognizes only these
typed command names, and the radio adapter emits only the mapped CAT in the
[source-backed interface specification](../docs/elecraft-kx2-kx3-interface.md#m1-cat-allowlist):

| Typed command | CAT owned by radio adapter | Exact result shape |
| --- | --- | --- |
| `radio_session_normalize` | `AI0;`/`AI;`, `K20;`/`K2;`, `K30;`/`K3;` | `{type, auto_information:"off", k2_mode:"off", k3_extended_mode:"off"}` |
| `radio_identify` | `OM;` | `{type, profile, product_code, option_flags}` |
| `radio_firmware_read` | `RVM;`, optional `RVD;` | `{type, main, dsp}` |
| `radio_vfo_a_read` | `FA;` | `{type, frequency_hz}` |
| `radio_vfo_a_set` | validated `FAxxxxxxxxxxx;`, then `FA;` | `{type, frequency_hz, query_verified:true}` |
| `radio_operating_state_read` | `IF;` | `{type, frequency_hz, tx_state}` |
| `radio_mode_read` | `MD;` | `{type, mode}` |
| `radio_tx_state_read` | `TQ;` | `{type, tx_state}` |

`radio_profile_select` selects only `kx2` or `kx3`, releases and mutes first,
invalidates the session, and requires new negotiation after profile validation.

The normative, machine-readable command and result schemas are
[`schema/v0-radio.schema.json`](schema/v0-radio.schema.json). Every radio command and
result has `additionalProperties: false`; there are no implicit, profile-specific, or
platform-specific members. In summary:

- all read and normalize commands are exactly `{type}`; `radio_vfo_a_set` additionally
  carries non-negative integer `frequency_hz`, and `radio_profile_select` additionally
  carries `profile: "kx2"|"kx3"`;
- `profile` is `"kx2"` or `"kx3"` and is inseparably correlated with
  `product_code`: KX2 is exactly `1`, KX3 is exactly `2`, and any mismatch is
  `radio_control_fault`; a mismatch marks the selected profile `faulted`, and recovery
  requires explicit profile selection, successful adapter validation, a fresh
  protocol session, and a fresh host-route report;
  `option_flags` is a unique array drawn from `"a"`, `"p"`, `"f"`, `"t"`, `"b"`,
  `"x"`, and `"i"`, corresponding to the documented `OM APF---TBXI0n` positions;
- firmware `main` is an `NN.NN` string and `dsp` is either the same string shape or
  JSON `null` when the optional query is unavailable;
- `mode` is one of `"lsb"`, `"usb"`, `"cw"`, `"fm"`, `"am"`, `"data"`,
  `"cw_reverse"`, or `"data_reverse"`;
- observational `tx_state` is `"receive"` or
  `"transmit_or_pseudo_transmit"`—never PTT authority;
- the adapter validates the entire fixed-width `IF` response but exposes only its
  allowlisted `frequency_hz` and observational `tx_state` fields in v0.0; and
- the profile-select result is exactly `{type, profile, state:"validating",
  session_invalidated:true}`. Profile validation is an adapter/service event, not CAT
  exposed to the host, and a fresh session is mandatory afterward.

Before any radio I/O, the implementation MUST:

- recognize the exact typed command;
- validate every field and selected-profile constraint;
- validate `radio_vfo_a_set.frequency_hz` against the profile and operator policy; and
- prove that the mapping contains no command that enters, sustains, tunes, keys, or
  otherwise requests transmit.

`raw_cat`, `TX`, `SWT`/`SWH` XMIT or TUNE, `KY`, power, VOX, mode writes, menu writes,
baud writes, and every unlisted command are `unsupported_radio_operation` with
`radio_io_attempted: false`. `IF` and `TQ` are observations only. They are neither PTT
authority nor substitutes for sensed `PTT OUT`.

A timeout, `?;`, unsolicited bytes, malformed or inconsistent response, model
mismatch, or query-after-set mismatch is `radio_control_fault`. During an active lease
it releases and latches first-cause `FAULT_LOCKOUT` without waiting for CAT recovery.

## Health and status snapshot

Every status has `type`, selected `v`, `device_id`, current `boot_id`, nullable current
`session_id`, `status_seq`, and `device_time_ms`. It then reports independent domains:

```json
{
  "health": {
    "ble_link": "connected",
    "protocol_session": "active",
    "host_usb_audio_route": "healthy",
    "device_usb_audio": {
      "aggregate": "healthy",
      "configured": true,
      "tx_stream": "active",
      "clock": "healthy",
      "buffers": "healthy",
      "converter": "healthy"
    },
    "radio_profile": "ready"
  },
  "radio": {
    "profile": "kx2",
    "observed_tx": "receive"
  },
  "ptt": {
    "commanded": "inactive",
    "ptt_out": "inactive",
    "inhibit": "closed",
    "owner": null,
    "lease_deadline_ms": null,
    "continuous_started_ms": null,
    "continuous_elapsed_ms": 0,
    "safety_state": "receive_safe",
    "last_release": {"code": "operator_release", "at_ms": 4100},
    "first_fault": null
  }
}
```

Allowed high-level health values are:

- `ble_link`: `connected` or `disconnected`;
- `protocol_session`: `none`, `negotiating`, `active`, or `faulted`;
- `host_usb_audio_route`: `unknown`, `healthy`, or `unhealthy`;
- `device_usb_audio.aggregate`: `unknown`, `healthy`, or `unhealthy`;
- `radio_profile`: `none`, `validating`, `ready`, or `faulted`;
- `ptt.commanded`: `inactive` or `active`;
- `ptt.ptt_out`: `unknown`, `inactive`, or `active`;
- `ptt.inhibit`: `unknown`, `open`, or `closed`; and
- `ptt.safety_state`: `receive_safe`, `tx_active`, or `fault_lockout`.

An owner, when present, is exactly `{boot_id, session_id, lease_id, intent_id}`.
`lease_deadline_ms` is an absolute device-monotonic deadline. The host derives
remaining time only from the same snapshot's `device_time_ms`; it never sends that
calculation back as authority.

`ptt_out` is the radio-side sensed node downstream of the controllable output and
independent inhibit. It does not prove RF output or the radio's full transmit state.
`commanded` and `ptt_out` MUST never be collapsed into one “transmitting” field.

`first_fault` is `{fault_id, code, at_ms, op_id}` with nullable `op_id`. Once latched,
later symptoms do not replace its code or time. Explicit successful recovery clears
the active first fault while retaining it as `last_fault` in bounded diagnostics.

Host route health is a claim from the host core, while device USB health is measured by
the device audio service. They remain separate. Audio bytes never appear in this
protocol and cannot assert PTT. Host route loss stops host renewal; when the report
arrives, it also denies or releases on-device. A detected required device-audio fault
releases within `T_RELEASE_MAX`; BLE may stay active for status and recovery.

BLE loss and protocol-session loss do not imply USB media loss. USB receive media may
continue after control failure. Conversely, a USB/audio fault does not imply BLE loss.

## PTT commands and safety state

All deadlines below are firmware-monotonic and owned by the dedicated safety service:

| Symbol | Value |
| --- | --- |
| `T_LEASE_MAX` | 500 ms |
| `T_RENEW_TARGET` | 250 ms host target only |
| `T_RELEASE_MAX` | 100 ms after expiry or detected safety event |
| `T_CONTINUOUS_MAX` | 60,000 ms from first assertion in an intent epoch |
| `T_REARM_MIN` | 1,000 ms continuously receive-safe after continuous-cap release |
| `T_WATCHDOG_RELEASE_MAX` | 500 ms after safety-heartbeat loss |

Host timestamps, wall clock, BLE supervision, app callbacks, background scheduling,
response delivery, release acknowledgement, and reconnect MUST NOT move a device
deadline.

### Intent, acquire, and grant

`ptt_intent_begin` carries a fresh `intent_id`. It records explicit current operator
intent but grants no authority. It is denied during lockout or when another intent is
open.

`ptt_acquire` carries that `intent_id` and `requested_ms` in `1..500`. It is accepted
only when:

- the request/session/sequence is current;
- safety state is `receive_safe` with no latched fault;
- host route is reported healthy and required device audio is healthy;
- BLE and protocol session are healthy;
- the selected profile is ready and binds exactly one hardware-PTT path;
- inhibit is closed; and
- sensed `PTT OUT` is inactive.

The safety service chooses `granted_ms <= min(requested_ms, 500)`, creates `lease_id`,
sets `lease_deadline_ms = accepted_at_ms + granted_ms`, starts the continuous timer on
first assertion, and reports command and sense separately. A grant never promises that
the radio emitted RF.

Stable acquire denials include `intent_required`, `lease_active`, `inhibit_open`,
`ptt_out_not_inactive`, `host_route_unhealthy`, `device_audio_unhealthy`,
`profile_not_ready`, `control_unhealthy`, and `fault_lockout`.

### Renew

`ptt_renew` carries the exact `intent_id`, `lease_id`, and `requested_ms`.
Acceptance requires the current owner tuple, every acquire precondition, the next
operation, and `accepted_at_ms < lease_deadline_ms`.

The new deadline is `accepted_at_ms + granted_ms`; it never adds to the old deadline.
An exact duplicate returns the cached old response and cannot extend it. A renew
accepted at device time `D` grants no authority after `D + 500` and never beyond the
continuous cap.

A renewal received at or after expiry is `lease_expired`. It cannot revive the lease.
The expired intent is closed; a fresh operator action and fresh acquire are required.

### Release and expiry

`ptt_release` carries `intent_id`, nullable `lease_id`, and a stable host reason. It is
idempotent at the logical operation layer. The safety service first requests output
inactive and mutes TX audio, invalidates the lease, closes the intent epoch, then
responds with current command/sense status. A missing response cannot keep authority
alive. Requesting release changes `ptt.commanded`; it MUST NOT fabricate a change to
the independently sampled `ptt.ptt_out`. Until a later sense sample reports inactive,
the device cannot claim that physical release succeeded. Sensed active after the
release bound latches `output_stuck_active`.

Expiry schedules inactive output before ordinary work, records `lease_expired`, and
releases within 100 ms when controllable. Detected BLE, session, audio, profile,
inhibit, CAT, or sensing faults request release immediately. Silent BLE loss still
cannot outlive the deadline plus the release bound.

### Continuous cap, lockout, rearm, and recovery

At 60,000 ms from first assertion in one `intent_id`, the safety service:

1. schedules PTT inactive and mute;
2. invalidates the lease;
3. enters `fault_lockout` with first cause `continuous_cap`;
4. rejects acquire and renew; and
5. starts the receive-safe interval only after command is inactive, sensed `PTT OUT`
   is inactive, inhibit and required health are safe, and host release intent arrived.

Any unsafe route, device-audio, inhibit, sensed-output, profile, BLE-link, or protocol
session interval resets the continuous-cap rearm timer to zero. Restoring the last
unsafe input starts a new full 1,000 ms interval; elapsed safe time from before the
interruption never counts. A `faulted` protocol session is unsafe: it must be replaced,
and the replacement session must re-report host-route health, before rearm can start.
`safety_recover` never changes a faulted session back to active.

Continuous-cap recovery requires all of:

1. `ptt_release` for the capped `intent_id`;
2. 1,000 uninterrupted device-monotonic milliseconds satisfying receive-safe inputs;
3. `safety_recover` carrying the current `fault_id`; and
4. a later `ptt_intent_begin` with a fresh `intent_id`.

Reconnect, session replacement, reboot, or a new `op_id` is not operator action and
does not bypass those conditions. A reboot clears volatile authority, creates a new
`boot_id`, and has no session or intent; it cannot resume the capped epoch.

Other lockouts require the cause to clear, sensed inactive output, and
`safety_recover` with the current `fault_id`. Stale or wrong recovery identities are
denied. Recovery never asserts PTT. A physically active stuck output cannot be called
receive-safe; the operator must open the independent TX inhibit or disconnect the
radio.

### Sensing and first cause

- Commanded active with sensed inactive: request inactive, mute, do not retry, and
  latch `output_failed_to_assert`.
- Commanded inactive with sensed active: reassert inactive, mute, urgently report
  `output_stuck_active`, and remain locked out. Firmware cannot claim release.
- Opening inhibit denies acquire and releases an active lease. If sensing follows the
  expected inhibited state, the release reason is `inhibit_open`; a contradictory
  sense becomes the first mismatch fault.
- A later symptom never overwrites the latched first cause.

## Other commands

The v0.0 command set is:

```text
status_read
host_audio_route_report
radio_profile_select
radio_session_normalize
radio_identify
radio_firmware_read
radio_vfo_a_read
radio_vfo_a_set
radio_operating_state_read
radio_mode_read
radio_tx_state_read
ptt_intent_begin
ptt_acquire
ptt_renew
ptt_release
safety_recover
```

`host_audio_route_report` contains `health: healthy|unhealthy|unknown` and an optional
host-local diagnostic token that is opaque to firmware. It has no samples and no host
timestamp. `status_read` causes a current status read/result but no state transition.

Unknown commands are `unsupported_command`. Syntactically valid but unsupported
commands consume their ordered operation and have `safety_effect: none` unless they
purport to be raw or keying-capable radio operations, which are rejected before radio
I/O and recorded as a security/safety diagnostic.

## Stable release, fault, and error codes

Release and fault codes used in v0.0 are:

```text
operator_release
lease_expired
continuous_cap
ble_disconnect
session_replaced
protocol_fault
host_route_unhealthy
device_audio_unhealthy
profile_fault
profile_change
radio_control_fault
inhibit_open
output_failed_to_assert
output_stuck_active
watchdog_reset
boot_or_update
```

Protocol and operation error codes are:

```text
malformed
message_too_large
transport_limit_too_small
unsupported_version
missing_capability
stale_boot
wrong_session
altered_duplicate
stale_operation
out_of_order
session_exhausted
unsupported_command
unsupported_radio_operation
invalid_argument
intent_required
intent_active
lease_active
lease_not_found
lease_expired
inhibit_open
ptt_out_not_inactive
host_route_unhealthy
device_audio_unhealthy
profile_not_ready
control_unhealthy
radio_control_fault
fault_lockout
rearm_incomplete
wrong_fault
```

New minor versions may add codes. Unknown codes are treated as non-retryable safety
errors; they never justify automatic acquire or renewal.

## Conditional USB MIDI adapter

USB MIDI is not implemented or selected by v0.0. If issue #18 triggers the ADR 0003
fallback, its adapter MUST transport the exact logical UTF-8 JSON messages, preserve
message boundaries and ordered delivery, and supply runtime direction limits to the
same protocol/session core. It MUST define its own MIDI framing elsewhere and MUST NOT
reuse the GATT fragment envelope by implication.

Transport change always replaces the session receive-safely. A USB MIDI connection,
like a BLE connection, is not PTT authority.

## Conformance artifacts

- [`vectors/v0.json`](vectors/v0.json) contains machine-readable fragmentation and
  semantic scenarios.
- [`schema/v0-radio.schema.json`](schema/v0-radio.schema.json) defines the exact typed
  radio command and result objects.
- [`conformance.py`](conformance.py) is a Python-standard-library harness with a
  virtual monotonic clock and no Apple or BLE framework dependency.

Run:

```sh
python3 protocol/conformance.py protocol/vectors/v0.json
```

The vectors cover hello/session negotiation and replacement, limit/version/capability
denials, negotiated fragment sizes, normal acquire/renew/release, exact and altered
duplicates, stale/replayed/out-of-order/malformed/wrong-session operations, expiry,
replacement/reconnect/boot identity, independent transport and audio health, inhibit,
command/sense mismatches including stuck output at release, exact typed radio results,
successful and contradictory profile/product identity responses, strict-parser
rejection of a negative frequency, command-level denial of a nonnegative out-of-schema
frequency before radio I/O, session exhaustion, profile selection, raw and
keying-capable CAT denial before radio I/O, radio faults, continuous-cap lockout,
faulted-session and other interrupted rearm, recovery, and first-cause preservation.

The harness is executable evidence that the contract can be consumed without Swift,
Core Bluetooth, AVFAudio, or a fixed Apple MTU. It is not production firmware and does
not claim physical KX2/KX3 behavior.

## Intentionally deferred

The following are not v0.0 fields or guarantees:

- BLE pairing, bonding, application authentication, authorization, or privacy policy;
- firmware update transfer and rollback protocol;
- USB Audio descriptors, formats, levels, device-audio thresholds, or power topology;
- GATT connection interval, PHY, supervision timeout, throughput, latency, or a fixed
  MTU;
- USB MIDI framing or implementation;
- arbitrary diagnostics export, raw CAT, a universal radio model, additional profiles,
  or additional PTT owners;
- unpublished radio electrical values or a hardware selection;
- persistent transmit intent, remote/cloud control, or internet operation; and
- an Android compatibility claim.

M1 may negotiate larger sizes and add bounded diagnostics without changing v0.0
meaning. Issue #10 owns descriptor/audio-health choices; #13 and #18 retain all
human-required physical and current-device evidence boundaries.

Protocol implementations and fixtures in this directory are licensed under
Apache-2.0.
