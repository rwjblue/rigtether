# 0004 — Bound transmit authority with device-enforced leases

- Status: Accepted
- Date: 2026-07-29

## Context

RigTether PTT crosses host software, BLE, firmware, an electrical output, a harness, and
the radio. iOS execution can be suspended or terminated, and Bluetooth link-loss
detection is not an application safety deadline. A connected host, a live audio route,
or a previously accepted level command therefore cannot safely hold transmit.

The feasibility baseline needs exact semantics before architecture and protocol work.
It also needs a recovery boundary for an output fault that firmware cannot clear. The
decision must not select a production switching component or claim unmeasured KX2/KX3
behavior.

## Decision

Firmware is the sole RigTether authority for one normally open hardware PTT output.
CAT operations that enter or sustain transmit are excluded from the feasibility
surface, and audio can never assert PTT.

Transmit authority is a lease owned by the current `(boot_id, session_id, lease_id)`.
A grant or renewal expires no more than 500 ms after firmware accepts it, using the
device monotonic clock. The host targets renewal at 250 ms. Expiry or a detected safety
event releases the controllable output within 100 ms. An independent watchdog resets
to the passively inactive output within 500 ms after loss of the safety-service
heartbeat.

Renewal sets a new deadline relative to acceptance; it never adds time. Exact duplicate
operations are idempotent and do not extend authority. Stale, altered, out-of-order, or
wrong-session operations cannot assert PTT. A new boot or session invalidates all prior
authority, and transmit intent is never persisted or restored.

One operator-intent epoch may hold PTT for at most 60 seconds regardless of renewal.
At the cap, the interface releases and locks out transmit. Rearm requires explicit
release intent, one continuously observed receive-safe second, and a new operator
action. Reconnect or power cycle does not itself restore authority.

Hardware provides inactive reset/power bias, a normally open output, and an independent
series TX inhibit. The radio-side PTT node is sensed downstream of the controllable
switch and inhibit. Local and protocol indication use that sensed node rather than the
commanded state and are labeled `PTT OUT`, because they do not prove RF output or the
radio's complete transmit state.

The detailed state model, fault responses, enforcement allocation, test matrix, source
classification, and unresolved risks are normative in
[Hardware and transmit safety](../hardware-safety.md).

## Alternatives considered

- **A level PTT command held until release:** rejected because a crash, stale state, or
  lost release can sustain transmit indefinitely.
- **Release only on BLE disconnect:** rejected because connection supervision and host
  callbacks do not provide the required application bound.
- **Host-only timeout:** rejected because the app can be suspended, terminated, or
  unable to communicate.
- **CAT PTT plus hardware PTT:** rejected for the feasibility baseline because two
  transmit mechanisms create ambiguous ownership and recovery.
- **Renewable short lease with no continuous cap:** rejected because one stuck renewal
  loop could sustain transmit indefinitely.
- **Command-state indication:** rejected because it can report safe while the output is
  stuck active, or report active while the inhibit has physically opened the path.
- **Firmware release without physical inhibit:** rejected because firmware cannot open
  a shorted output device or conductor.

## Consequences

- The v0 protocol must carry boot, session, lease, operation, deadline, output-sense,
  inhibit, and fault identity without weakening the timing and replay rules.
- Architecture must give the firmware safety service a monotonic timer, watchdog,
  output feedback, and independent control/audio health inputs.
- M1 hardware must preserve passive receive bias, a series inhibit, output sensing,
  protection, and test points without yet choosing a production component.
- Host adapters stop renewal on any loss of explicit intent or platform certainty, but
  device expiry remains the guarantee.
- M1 can test the invariants with a radio-disconnected electrical fixture. Real-radio
  and RF evidence remains human-required.
- A shorted output is an explicit residual risk recovered through the independent
  physical inhibit. `PTT OUT` is not an RF or regulatory claim.
- The 500 ms lease, 60 s continuous cap, and other numeric bounds are feasibility
  requirements. Changing them requires measured evidence and an explicit ADR update.

## Evidence

Sources were accessed 2026-07-29. Manufacturer and platform behavior is documented in
the [hardware-safety evidence table]. The selected architecture and numeric bounds are
engineering decisions. No bench work, component qualification, radio transmission, or
regulatory analysis was performed.

[hardware-safety evidence table]: ../hardware-safety.md#evidence-and-source-classification
