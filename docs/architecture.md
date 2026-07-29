# Architecture hypothesis

This page records the current shape of the problem. It is not an accepted schematic or
complete system architecture. The phone transport is a settled input from
[ADR 0003](decisions/0003-usb-audio-plus-ble-control.md).

## Working model

```text
┌────────────────────────────┐
│ Host application           │
│ iOS first; Android future  │
│                            │
│ platform audio adapter     │
│ protocol/control adapter   │
└──────────┬─────────────────┘
        │
        │ USB-C: class-compliant USB audio
        │ BLE: versioned GATT control service
        ▼
┌────────────────────────────────────┐
│ RigTether interface                │
│                                    │
│ host transport                     │
│ control firmware                   │
│ audio conversion and level control │
│ CAT electrical interface           │
│ hardware PTT / TX inhibit          │
│ watchdog and fault handling        │
└──────────────┬─────────────────────┘
               │ replaceable harness
               ▼
┌──────────────────────┐
│ Radio adapter        │
│ KX2/KX3 first        │
│                      │
│ phones / mic / ACC   │
└──────────────────────┘
```

## Boundaries

- **Host audio transport:** a class-compliant USB Audio function presents one or more
  receive and transmit streams through the host audio stack. The iOS adapter uses
  AVFAudio. M1 selects and validates the exact class revision, descriptors, channel
  count, sample formats, and power behavior on iPhone while preserving a documented
  Android path or escalating an incompatibility for owner decision.
- **Host control transport:** a custom BLE GATT service carries discovery,
  capabilities, state, commands, bounded PTT leases, and errors. The host is the BLE
  central/GATT client and the interface is the peripheral/GATT server. Core Bluetooth
  owns the iOS implementation, not the device contract.
- **Host adapters:** own platform audio routing, permissions, lifecycle, and BLE API
  behavior. They translate those events into the versioned transport and safety model
  without making platform callbacks or framework types part of the device protocol.
- **Control core:** owns protocol state, watchdogs, PTT arbitration, and radio profile.
- **CAT adapter:** converts versioned host operations or safe passthrough into the
  radio's documented CAT protocol.
- **Audio front end:** performs DC blocking, attenuation/gain, filtering, protection,
  and any required isolation.
- **PTT path:** independent fail-open output with explicit ownership and timeout.
- **Harness:** owns radio connector fan-out, shielding, strain relief, and optional
  identification.

## Invariants that architecture must preserve

- No transmit on boot, reset, firmware update, host disconnect, or watchdog expiry.
- Audio output alone must not unexpectedly assert PTT unless an explicitly selected
  and bounded VOX mode exists.
- Radio-specific pin assignments never appear as generic host API guarantees.
- Apple framework types, callback ordering, background execution, and audio-session
  behavior never become device-side protocol guarantees.
- Firmware and host versions negotiate capabilities rather than silently assuming them.
- The bench prototype remains observable: test points, logs, and simulators are first-
  class requirements.

## Minimum prototype transport contract

ADR 0003 fixes only the boundary needed by architecture and protocol work:

- USB Audio and BLE control are independent logical transports. Either may connect or
  fail without implying the state of the other.
- Audio is never a control channel. Audio presence, samples, or route activation cannot
  assert PTT.
- The BLE service exposes a discoverable service identity, protocol version and
  capabilities, a reliable command/response path, asynchronous status including the
  actual PTT output, and explicit connection/session identity.
- Every transmit request is a bounded lease enforced by the interface. Loss of BLE,
  expiry, reset, malformed input, or a new session returns the interface to receive.
- Message boundaries and sizes respect the negotiated GATT/ATT limits. The v0 protocol
  issue owns payloads, characteristic layout, ordering, errors, and test vectors.
- The iOS app may use the documented audio and `bluetooth-central` background modes
  only for their intended work. Safety does not depend on indefinite host execution on
  iOS, Android, or another platform.

The single-tether goal remains intact: one USB-C cable carries audio and any supported
power, while BLE is wireless control. Pairing, reconnect, coexistence, latency, and
power are explicit M1 measurements rather than assumed properties.

## Open M0 questions

- Which USB Audio Class topology and sample formats should the M1 hardware expose?
- Which conservative descriptor and format choices preserve Android USB Audio host
  interoperability without weakening the iPhone proof?
- What exact voltage, impedance, bias, grounding, and timing requirements apply to KX2
  and KX3 ports?
- Should the protocol expose raw CAT, typed capabilities, or both?
- Which safety functions must be hardware-enforced rather than firmware-enforced?

The M0 architecture decision closes these questions enough to authorize the bench
proof; it need not settle production component choices.
