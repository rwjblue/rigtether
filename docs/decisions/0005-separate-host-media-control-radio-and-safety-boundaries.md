# 0005 — Separate host, media, control, radio, and safety boundaries

- Status: Accepted
- Date: 2026-07-29

## Context

ADR 0003 selected class-compliant USB Audio for bidirectional media and BLE GATT for
control with independent link health. ADR 0004 made firmware the sole owner of one
normally open hardware-PTT path with bounded leases, a continuous-transmit cap,
watchdog release, an independent physical inhibit, radio-side output sensing, and
receive-safe fault behavior. The source-backed KX2/KX3 interface specification fixes
the radio-facing inputs and leaves unpublished electrical values to human measurement.

M1 needs an implementable architecture that joins those decisions without allowing
iOS framework behavior, audio samples, CAT operations, profile details, or a stale
transport state to become transmit authority. It must also preserve a credible Android
host path and allow hardware-independent testing before issue #13 supplies physical
evidence.

## Decision

Adopt the component and test boundaries in
[Feasibility architecture](../architecture.md).

The host consists of a platform-neutral host core behind platform adapters. The iOS
adapter solely owns AVFAudio, Core Bluetooth, permissions, route and lifecycle
callbacks, and background behavior. A future Android adapter implements the same
event/value ports and consumes the same protocol bytes, capabilities, safety semantics,
and fixtures. No Apple framework object, callback order, state restoration, MTU, or
background guarantee is part of the device contract.

USB Audio and BLE GATT remain independent state machines. Device firmware separately
owns USB configuration/stream/clock/buffer health, BLE link health, and negotiated
protocol-session health. Audio content never controls PTT. A required media fault can
deny or release transmit while BLE remains available for receive-safe control,
diagnostics, and recovery; a BLE or protocol fault releases transmit while USB receive
media may continue.

Device firmware is divided into transport adapters, a protocol/session core, an audio
service, a radio service, and a dedicated high-priority PTT safety service. The safety
service alone owns the controllable PTT output, firmware monotonic lease and
continuous-cap clocks, safety state, watchdog heartbeat, inhibit/output-sense inputs,
and lockout. Other services supply qualified preconditions or faults but cannot assert
PTT. Release preempts ordinary audio, CAT, protocol, and diagnostic work and never
waits for CAT or host acknowledgement.

The radio service owns distinct KX2 and KX3 profiles and replaceable harnesses. Each
active profile binds exactly one normally open hardware-PTT path. Its CAT adapter
exposes only the typed allowlist from the KX2/KX3 specification and verifies sets with
queries. Raw CAT, CAT/audio keying, dual PTT paths, and use of observed `IF`/`TQ` state
as authority are excluded.

Hardware owns passive inactive bias, audio and CAT electrical conversion, protection
and test access, the normally open PTT output, an independent normally open series TX
inhibit, and radio-side sensing downstream of the output and inhibit. Issue #13 owns
the human-supervised measurements and measurement-dependent bench choices; the
architecture records unknowns rather than substituting typical values.

Startup, reset, update, profile change, session replacement, independent transport
loss, and recovery all begin by releasing and muting. New boots and sessions invalidate
old authority. Lockout recovery requires the cause to clear, sensed inactive output,
and an explicit recovery action; reconnect or power cycle never restores transmit
intent.

M1 descriptor and PCM choices should remain inside Android's documented USB Audio
Class 1 host-mode subset when compatible with the iPhone proof. A contradictory
requirement must be escalated with evidence, compatibility cost, a credible alternate
path, and an explicit owner decision. This constraint preserves implementability but
does not claim Android compatibility.

## Evidence

This decision synthesizes accepted, source-backed inputs; it does not add a physical
result. Sources were accessed for the input work on 2026-07-29.

| Accepted input | Architecture consequence |
| --- | --- |
| [ADR 0003](0003-usb-audio-plus-ble-control.md) and its Apple, Android, USB-IF, and Bluetooth SIG evidence | USB Audio carries media, BLE GATT carries control, link health stays independent, Apple behavior remains in the iOS adapter, and USB MIDI remains conditional |
| [ADR 0004](0004-bound-transmit-authority-with-device-enforced-leases.md) and the normative [hardware-safety contract](../hardware-safety.md) | One firmware safety service owns the normally open hardware-PTT path, monotonic leases, continuous cap, watchdog, inhibit, output sensing, and lockout |
| [KX2/KX3 interface specification](../elecraft-kx2-kx3-interface.md) and its revisioned Elecraft sources | KX2/KX3 profiles and harnesses stay separate, CAT is typed and non-keying, exactly one PTT path is selected, and unpublished values remain measurement-required |

No MCU, audio converter, PTT circuit, USB descriptor, harness, or radio behavior was
built or measured for this decision. Issue #13 remains the evidence boundary for
human-supervised physical results.

## Alternatives considered

- **Put all host behavior in the Swift probe:** rejected because AVFAudio, Core
  Bluetooth, lifecycle, and callback assumptions would leak into protocol and safety
  behavior and make a future Android adapter needlessly incompatible.
- **Treat USB and BLE connection as one session:** rejected because ADR 0003 requires
  independent link health and either transport can remain useful after the other
  fails.
- **Let the general control loop own PTT:** rejected because audio, CAT, BLE, and
  diagnostics work could delay deadlines or create multiple software owners. ADR 0004
  requires one device-enforced authority and an independent watchdog boundary.
- **Use CAT or audio state as a second keying path:** rejected by ADR 0004. Multiple
  keying mechanisms make authority, release, and recovery ambiguous.
- **Create one generic Elecraft profile and harness:** rejected because documented and
  unknown KX2/KX3 electrical behavior differs. Radio-specific details belong at the
  profile/harness edge.
- **Wait for issue #13 measurements before defining architecture:** rejected because
  configurable conversion, profile, harness, fixture, and unknown-value boundaries can
  be fixed without selecting measurement-dependent circuits. Waiting would create an
  unnecessary dependency cycle.
- **Select MCU, codec, and production topology now:** rejected because architecture
  needs capability and observability requirements, not premature component choices.
- **Design USB MIDI into the primary control path:** rejected because it is only the
  evidence-triggered fallback in ADR 0003.

## Consequences

- Issue #6 can define one platform-neutral protocol and conformance-vector set while
  preserving the accepted safety semantics.
- M1 firmware must schedule a dedicated safety service and must expose separate media,
  control, profile, inhibit, output-sense, and fault diagnostics.
- Issue #10 selects development hardware, exact descriptors/formats, and audio-health
  thresholds against explicit requirements and the Android-preservation rule.
- Issues #9, #11, #12, #13, and #18 can use simulators, transcript fixtures, loopback,
  and a radio-disconnected PTT fixture before any live-radio evidence.
- The iOS probe may be narrow, but its adapter boundary and fixtures cannot define
  Apple-only device behavior.
- The architecture adds coordination between independent media and control state
  machines. That complexity is intentional and must be visible in logs and tests.
- Production component selection, PCB/enclosure work, public SDK stability, Android
  implementation, final update/security design, and all unpublished radio electrical
  values remain deferred.
- No physical result is claimed. Real-radio and measurement-dependent evidence remains
  human-required in issue #13.
