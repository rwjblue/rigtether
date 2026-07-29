# Architecture hypothesis

This page records the current shape of the problem. It is not an accepted schematic or
transport decision.

## Working model

```text
┌──────────────────────┐
│ iPhone application   │
│                      │
│ audio engine         │
│ RigTetherKit          │
└───────┬──────────────┘
        │
        │ candidate: USB audio + BLE control
        │ alternatives remain open in M0
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

## Candidate boundaries

- **Host audio transport:** presents receive and transmit streams through an iPhone-
  compatible mechanism.
- **Host control transport:** discovery, capabilities, state, commands, PTT, and errors.
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
- Firmware and host versions negotiate capabilities rather than silently assuming them.
- The bench prototype remains observable: test points, logs, and simulators are first-
  class requirements.

## Open M0 questions

- Can a broadly distributable iOS app access the chosen control transport without MFi,
  restricted entitlements, or private APIs?
- Which USB Audio Class topology and sample formats are most compatible and useful?
- Is BLE control the best separation, or can a single composite USB device satisfy the
  product and distribution constraints?
- What exact voltage, impedance, bias, grounding, and timing requirements apply to KX2
  and KX3 ports?
- Should the protocol expose raw CAT, typed capabilities, or both?
- Which safety functions must be hardware-enforced rather than firmware-enforced?

The M0 architecture decision closes these questions enough to authorize the bench
proof; it need not settle production component choices.
