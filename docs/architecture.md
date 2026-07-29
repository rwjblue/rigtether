# Feasibility architecture

This is the accepted M0 component architecture. It refines
[ADR 0003](decisions/0003-usb-audio-plus-ble-control.md) and
[ADR 0004](decisions/0004-bound-transmit-authority-with-device-enforced-leases.md)
without changing either decision. [ADR 0005](decisions/0005-separate-host-media-control-radio-and-safety-boundaries.md)
records the boundary decision.

The architecture authorizes an instrumentable M1 bench proof. It does not select a
production MCU, codec, circuit, PCB, enclosure, public SDK, or unpublished electrical
value.

## System model

```text
┌───────────────────────────────────────────────────────────────────────┐
│ Host application                                                     │
│                                                                       │
│  operator intent + UI ── platform-neutral host core                  │
│                              │                                        │
│  iOS adapter                  │ future Android adapter                 │
│  AVFAudio + Core Bluetooth   │ platform audio + BLE APIs             │
└──────────────┬───────────────┴──────────────────┬─────────────────────┘
               │ USB Audio Class                  │ BLE GATT
               │ bidirectional media              │ versioned control
               │ independent health               │ independent health
┌──────────────▼──────────────────────────────────▼─────────────────────┐
│ RigTether firmware                                                    │
│                                                                       │
│  USB audio function ── audio service        BLE transport adapter     │
│         │             route/clock/buffer          │                   │
│         │             health + mute               ▼                   │
│         │                                  protocol/session core      │
│         │                                    │          │             │
│         │                            typed CAT│          │preconditions│
│         │                                    ▼          ▼             │
│         │                              radio service  PTT safety svc  │
│         │                              + profiles     monotonic clock │
│         │                                    │        + watchdog      │
└─────────┼────────────────────────────────────┼──────────┼─────────────┘
          │                                    │          │
┌─────────▼────────────────────────────────────▼──────────▼─────────────┐
│ Replaceable bench hardware                                            │
│ audio conversion │ CAT electrical I/O │ normally-open PTT output     │
│ protection/test points                  │ series TX inhibit │ sensing │
└───────────────────────────────┬───────────────────────────────────────┘
                                │ replaceable, profile-specific harness
                         ┌──────▼──────┐
                         │ KX2 or KX3 │
                         └─────────────┘
```

Arrows into the PTT safety service provide observations, preconditions, or release
events. They do not grant another component authority to energize PTT.

## Component ownership

Every cross-layer responsibility has one primary owner. Other components may observe
or request behavior only through the named boundary.

| Component | Primary owner | Contract and exclusions |
| --- | --- | --- |
| Operator intent and application workflow | Host application | Produces a fresh press/hold/release intent epoch. It never persists or restores transmit intent. UI state is not device authority. |
| iOS host adapter | Swift/iOS layer | Owns AVFAudio, Core Bluetooth, permissions, audio-session and route changes, lifecycle and background callbacks, and conversion of platform events into host-core events. No Apple framework type, callback order, restoration behavior, MTU assumption, or background guarantee crosses the adapter. |
| Future Android adapter boundary | Future Android layer | Must be able to implement the same host-core ports with Android audio and BLE APIs. It consumes the same byte protocol, capabilities, fixtures, and safety semantics; it does not reproduce AVFAudio or Core Bluetooth behavior. |
| Platform-neutral host core | Host library contract | Owns device discovery state, capability negotiation, session establishment, ordered operations, typed radio operations, status reduction, and the 250 ms lease-renewal target. It stops renewing on intent, route, lifecycle, or control uncertainty. Firmware deadlines remain the safety guarantee. |
| USB Audio function | Device USB/audio firmware | Exposes class-compliant full-duplex media and reports device-observed configuration, stream, clock, and buffer health to the audio service. It carries no control or PTT meaning. |
| BLE transport adapter | Device BLE firmware | Exposes the RigTether GATT service, negotiates ATT limits, and moves bounded protocol messages. BLE connection state alone is neither a session nor transmit authority. |
| Protocol and session core | Host-neutral device firmware plus the [v0 contract](../protocol/README.md) | Owns version/capability negotiation, session replacement, operation ordering/idempotency, typed commands, status and errors. It publishes and validates the current `boot_id` supplied by the boot/update coordinator but never creates or rotates it. The contract owns its wire representation and vectors. |
| Audio service | Device audio firmware | Owns media routing between USB and the conversion boundary, sample-format conversion, gain/mute control, and device-observed transmit-audio health. Silence is valid media; stream/clock/buffer failure is not. It can require release but cannot assert PTT. |
| Audio conversion boundary | Replaceable bench hardware plus its driver | Owns codec or converter attachment, DC blocking, filtering, bounded gain/attenuation, protection, and loopback/test access. ADR 0007 fixes the M1 development implementation/formats; measured radio values remain #13 evidence. |
| Radio service | Host-neutral firmware | Owns the selected radio profile, harness validation state, typed CAT adapter, and mapping of generic capabilities to KX2/KX3 behavior. It can deny or release PTT but cannot energize it directly. |
| CAT adapter | Radio service and reusable CAT core | Implements only the source-backed typed allowlist from the [KX2/KX3 specification](elecraft-kx2-kx3-interface.md), including query-after-set verification. Raw CAT and every command capable of entering or sustaining transmit are rejected before radio I/O. `IF` and `TQ` are observations, not PTT authority or substitutes for sensed `PTT OUT`. |
| Radio profiles | Radio service configuration | KX2 and KX3 are separate profiles. Each binds audio, CAT, settings, diagnostics, and exactly one normally open hardware-PTT path. Profiles contain documented values and explicit unknowns; they do not fill measurement-required fields with typical values. |
| Harness | Replaceable passive radio-side assembly | Owns connector fan-out, shielding, strain relief, protection placement, separate ground access, test access, and optional identification. Harness replacement does not change host or protocol semantics. |
| PTT safety service | Dedicated high-priority firmware service | Sole software owner of the controllable PTT output, safety state, monotonic lease and continuous-cap clocks, output command, inhibit and output-sense inputs, fault lockout, and safety event log. It consumes qualified health inputs and never waits for CAT or host acknowledgement to release. |
| Independent watchdog | Hardware watchdog configured by the safety service | Receives a heartbeat only after the safety service checks deadlines, session, profile, required health, inhibit, and sensed output. Its reset path exposes the passively inactive PTT state within ADR 0004's bound. |
| PTT electrical boundary | Replaceable bench hardware | Provides inactive reset/power bias, one normally open controllable output, current limiting/protection, a physically independent normally open series TX inhibit, radio-side sensing downstream of both, and test points on both sides of the inhibit. |
| Boot/update coordinator | Firmware and bootloader | Sole creator of one immutable `boot_id` per firmware boot, before any session or lease can exist. It releases and mutes before reset, profile changes, or update and rejects unsafe transitions. Bootloader and unconfigured pins rely on passive inactive hardware, never restored software intent. |
| Diagnostics service | Firmware with host tooling | Publishes bounded, versioned status and trace records without becoming a control backdoor. Desktop-only debug access may exist but is not part of the iPhone contract and cannot bypass the typed protocol or safety service. |

## Host-portable boundary

The host core is defined by events and values, not platform objects:

- audio route present/lost, selected format, stream requested, and route uncertainty;
- control transport available/lost and a sequence of received protocol bytes;
- explicit operator intent begin/held/released;
- negotiated protocol version, capabilities, active profile, and device health;
- session-scoped commands and status with monotonic device-relative durations; and
- lifecycle certainty lost/restored, where restoration always begins receive-safe.

The iOS adapter translates AVFAudio and Core Bluetooth behavior into those events. A
future Android adapter can translate its platform APIs into the same events. Neither
adapter exports platform errors, callback ordering, state-restoration tokens, or
background scheduling as protocol semantics.

M1 uses the ADR 0007 proposal: full-speed USB Audio Class 1.0 with one mono signed
16-bit 48 kHz PCM stream in each direction. That format is inside Android's documented
USB Audio Class 1 host-mode subset. Exact descriptors, terminals, endpoints, clock
discipline, and health thresholds are fixed as #18 validation inputs in the
[M1 development-platform specification](m1-development-platform.md). Until
representative-device testing, this is an implementability constraint, not an Android
or iPhone support claim.

## Independent transport and audio health

USB media and BLE control have separate state machines and diagnostics.

| Signal | Owner and source | Safety use | Test strategy |
| --- | --- | --- | --- |
| Host audio route health | Platform adapter, from current route and lifecycle certainty | Loss stops host renewals and is reported to the device when possible; device safety never waits for that report | Host-adapter unit tests with synthetic route/lifecycle events; iOS probe route-change tests in #11/#18 |
| Device USB media health | USB audio function and audio service, from configured state, active transmit stream, valid clocks, and bounded buffer/codec faults | Required for acquire and renewal; detected loss releases within `T_RELEASE_MAX` and mutes TX audio | Firmware fault injection for detach, clock loss, underrun/overrun, converter fault, and mute; USB loopback in #12/#18 |
| BLE link health | BLE transport adapter | Explicit disconnect immediately invalidates the session and releases; silent loss is still bounded by lease expiry | Simulated disconnect, blocked traffic, supervision delay, and reconnect; lease timing remains independent |
| Protocol session health | Protocol/session core, from negotiated current session and ordered valid traffic | Wrong-session, malformed, altered duplicate, or ordering fault cannot assert and releases/locks out as ADR 0004 requires | Platform-neutral [protocol vectors](../protocol/vectors/v0.json) and independent implementations |
| CAT/profile health | Radio service, from profile validation and typed CAT results | Invalid profile or CAT uncertainty denies acquire; an active safety-relevant fault releases without waiting for CAT | Radio-disconnected profile tests plus CAT transcript simulator in #9 |
| Inhibit and output health | PTT safety service, from hardware inputs | Inhibit denies assertion; commanded/sensed mismatch releases, latches, and reports `FAULT_LOCKOUT` | Radio-disconnected electrical fixture and forced faults in #13 |

Audio sample values, including silence, clipping, or arbitrary tones, can never assert
PTT. ADR 0007 selects the initial hardware-dependent health thresholds and buffering
budgets. Issues #12/#18 test them; no result may make sample content or host timing
transmit authority.

USB failure does not imply BLE failure: control, receive-safe CAT reads, diagnostics,
and recovery may remain available, but transmit is denied. BLE failure does not imply
USB failure: receive media may continue, while transmit authority expires or is
released. The UI displays the two health states separately.

## Control, capabilities, and profiles

The v0 protocol negotiates before accepting ordinary commands:

1. The device advertises a stable service identity and reports protocol-version range,
   `boot_id`, device capabilities, supported radio profiles, configured audio
   capabilities/health, and safety features.
2. The host selects a mutually supported protocol version and starts a fresh session.
   Session replacement first releases PTT and invalidates the prior session and every
   lease.
3. Both sides use runtime-negotiated GATT/ATT message limits. Payload semantics and
   fixtures remain reusable by the conditional USB MIDI transport adapter.
4. The host requests a radio profile explicitly. Firmware validates the profile and
   harness state before reporting it ready. An unknown or ambiguous profile is
   receive-safe.
5. Capabilities expose typed operations. Unsupported or non-allowlisted CAT operations
   fail locally before bytes reach the radio.

The boot/update coordinator creates `boot_id` exactly once for a firmware boot and
exposes it as immutable boot context. The safety service consumes that value as part of
every lease-owner tuple. The protocol/session core publishes it and rejects mismatches;
neither component creates, rotates, or independently caches a different boot epoch.

The normative [v0 control contract](../protocol/README.md) and
[ADR 0006](decisions/0006-use-framed-json-for-the-v0-control-contract.md) fix the M1
wire boundary. Logical messages are strict UTF-8 JSON. BLE uses runtime-sized
fragmentation across read-only hello, command, indicated response, and snapshot-status
characteristics. The protocol creates a fresh device-side session, enforces ordered
operations with non-evicting exact-duplicate results, and exposes all safety times only
on the device monotonic clock. A future USB MIDI adapter reuses logical messages but
does not inherit GATT framing.

For M1, session normalization sends and verifies `AI0`, `K20`, and `K30`. The typed CAT
surface is `OM` identification/options, `RVM` and optional `RVD` firmware reads, `FA`
read and query-verified set, plus read-only `IF`, `MD`, and `TQ`. The complete response,
model, fixed fields, and product-specific values are validated. Timeout, `?;`,
malformed, unsolicited, mismatched, or inconsistent results are explicit faults.
The v0 protocol names these as typed operations and contains no raw CAT field. Unknown,
unsupported, or keying-capable radio operations are rejected before radio I/O.

KX2 and KX3 use replaceable harnesses and distinct profiles even when a documented
command or connector is similar. KX3 may compare measured mic PTT with
`ACC2 IO LO=PTT`, but one profile enables exactly one path. `HI=PTT`, Key Out/Keyline,
the KX3 remote-power-on stimulus, CAT keying, and dual PTT paths are excluded. KX3
native inhibit may be defense in depth but never replaces the independent series
inhibit or radio-side sensed `PTT OUT`.

## Safety service and state

The normative safety semantics and timing remain in
[Hardware and transmit safety](hardware-safety.md). The architecture gives them this
execution boundary:

- the safety service initializes before USB, BLE, CAT, or audio work;
- a firmware-owned monotonic clock drives lease expiry, the 60 s continuous cap, the
  1 s rearm interval, and all safety timestamps;
- acquire checks the current session, explicit intent epoch, profile, required audio
  and control health, inhibit, sensed inactive output, and latched faults;
- only the safety service can change the controllable PTT output;
- release work preempts audio, CAT, diagnostics, and ordinary protocol work;
- the watchdog heartbeat proves a complete safety-service pass, not merely a live
  timer interrupt; and
- commanded state, sensed `PTT OUT`, inhibit, lease owner/deadline, continuous time,
  and release/fault reason are separately observable.

`PTT OUT` describes the sensed radio-side node. It is never labeled RF, on-air, or
complete radio transmit state.

## Lifecycle and fault flows

### Startup and negotiation

1. Passive hardware holds PTT open and TX audio muted while power and reset settle.
2. The boot/update coordinator creates one fresh `boot_id` before enabling any
   session, lease, or host-facing service. The value remains immutable until reset.
3. The safety service configures inactive output and inputs, consumes the current
   `boot_id`, starts its monotonic clock and watchdog checks, clears any persisted
   authority, and enters `RECEIVE_SAFE`.
4. Audio, BLE, CAT, profile, and diagnostics services start independently. Startup
   failures remain receive-safe and are reported when a control session becomes
   available.
5. A BLE connection negotiates capabilities and creates a fresh session. USB media may
   enumerate before or after it; neither ordering is assumed.

### Normal receive and transmit

- In receive, USB input media, typed CAT observations, and diagnostics may operate
  while PTT stays open and transmit audio remains muted or quiescent.
- A fresh operator action starts an intent epoch. The host requests acquire only after
  both host adapters are certain; firmware independently checks every device
  precondition.
- Firmware grants at most 500 ms, asserts only through the safety service, and reports
  commanded and sensed state separately. The host targets renewal at 250 ms.
- Release intent stops renewal immediately. Firmware requests inactive, invalidates
  the lease, mutes TX audio, verifies sensed output, and then reports the result.
  Missing acknowledgement cannot keep authority alive.

### Session replacement

A new session, reconnect, restored platform connection, or competing central first
releases and invalidates the old lease and session. It begins in `RECEIVE_SAFE` with no
restored operator intent. Old operations are rejected by boot/session/operation
identity and cannot extend or revive authority.

### Independent transport failures

- **USB detach, route loss, or device audio fault:** host stops renewal when able;
  device detection releases within `T_RELEASE_MAX`, and lease expiry is the outer
  bound if no earlier event arrives. TX audio mutes. BLE may continue diagnostics,
  safe CAT, and recovery.
- **BLE disconnect or protocol fault:** firmware releases immediately on a detected
  event; otherwise lack of valid renewal expires the lease. USB receive media may
  continue. Reconnect creates a new receive-safe session.
- **Both fail:** passive output, monotonic lease, and watchdog boundaries converge on
  `RECEIVE_SAFE`; recovery treats both transports independently.

### Reset, profile change, and firmware update

- Reset or watchdog expiry exposes passive inactive hardware. On the next boot, the
  boot/update coordinator creates a new `boot_id`; no session, lease, or intent is
  restored.
- Profile or harness change is allowed only after release and mute. It invalidates the
  session, validates the new KX2/KX3 profile and exactly one PTT binding, and requires
  a fresh negotiation. Ambiguity enters lockout.
- Update is rejected while output is sensed active. The update coordinator releases,
  mutes, invalidates the session, and requires the physical inhibit open for the M1
  procedure before bootloader entry. Bootloader, interrupted update, rollback, and
  first boot remain passively inactive. After a successful update, the rebooted
  coordinator creates a new `boot_id` and requires fresh negotiation.

### Fault lockout and recovery

Safety faults release and mute first, preserve the first-cause diagnostic, and reject
acquire/renew. Recovery requires the cause to clear, output sensed inactive, an
explicit recovery action, and a fresh session or profile validation where relevant.
The continuous-cap case additionally requires reported release intent, one continuous
receive-safe second, and a new operator action. Reconnect or power cycle alone never
restores transmit. If the output is physically stuck active, the operator opens the
independent TX inhibit or disconnects the radio; firmware cannot claim receive-safe.

## Observability and test architecture

The bench proof must be diagnosable without a radio and without unsafe backdoors.

### Structured diagnostics

Firmware records monotonic timestamps and stable reason codes for:

- boot, reset source, update state, protocol version, capability and profile changes;
- USB configured/stream/format/clock/buffer state and BLE link/session/ATT limits;
- host route claim and device-observed audio health as distinct values;
- operation identity, lease owner, grant/deadline, continuous time, and safety
  transitions;
- commanded PTT, sensed `PTT OUT`, inhibit, mismatch, watchdog, and lockout/recovery;
- typed CAT request/result/timeout/parse state without providing raw command injection;
  and
- simulator, fixture, firmware, host, profile, and harness revision identifiers.

Logs are bounded and must not block or delay release. A receive-safe diagnostics export
uses the versioned control surface or an explicitly desktop-only tool; neither can
write PTT hardware or bypass the CAT allowlist.

### Test points and loopback

The M1 hardware exposes power/ground domains, audio converter input/output, CAT
electrical input/output, controllable PTT before inhibit, radio-side PTT after inhibit,
inhibit state where sensed, and watchdog/reset observation. Separate audio, CAT, PTT,
radio chassis, and interface grounds remain accessible until #13 measurements justify
any connection.

Loopback and fixture support includes:

- USB audio digital and converter-side loopback with known media patterns;
- BLE transport fault injection and a host-neutral protocol conformance harness;
- deterministic CAT simulation with document-derived KX2/KX3 transcript fixtures,
  separately labeled from later human-captured evidence;
- a radio-disconnected PTT fixture for inactive bias, lease timing, watchdog, inhibit,
  sense, mismatch, and stuck-active tests; and
- recorded session transcripts reusable by Swift, Rust, and a future Android client.

### Responsibility-to-test map

| Responsibility | Primary verification before live radio |
| --- | --- |
| iOS adapter containment | Compile-time module boundary plus synthetic permission, route, lifecycle, BLE reconnect, and state-restoration tests; host/protocol fixtures contain no Apple types |
| Android path preservation | Build or parse fixtures with a non-Swift harness; compare ADR 0007 descriptors/formats with the Android UAC1 baseline; escalate any exception |
| USB Audio transport and conversion | Descriptor inspection, enumeration records, independent detach/clock/buffer fault injection, digital/analog loopback, format conversion and mute tests |
| BLE transport | Second GATT implementation or simulator, negotiated-size boundaries, blocked traffic, disconnect/reconnect, duplicate and ordering faults |
| Protocol/capabilities | [Version/capability/session vectors](../protocol/vectors/v0.json) consumed independently by Swift and the platform-neutral harness |
| CAT allowlist | Parser/encoder unit tests, forbidden-command pre-I/O checks, KX2/KX3 transcript replay, timeout and malformed-response injection from #9 |
| Profiles and harnesses | Configuration-schema tests, exactly-one-PTT assertion, unknown-value preservation, wrong/ambiguous profile denial, passive breakout continuity checks |
| Lease clock and continuous cap | Virtual monotonic-clock tests plus timestamped fixture traces at 500 ms lease, 100 ms release, 60 s cap, and 1 s rearm bounds |
| Watchdog and startup/update | Stall the complete safety heartbeat, reset/brownout/update fault injection, verify passive inactive output and new `boot_id` |
| Inhibit, sensing, mismatch, lockout | Radio-disconnected fixture opens inhibit and forces inactive/active sense disagreement; verify physical interruption, latch, status, and explicit recovery |
| End-to-end states | Deterministic simulator scenarios for every lifecycle flow above, followed by #13 human-supervised per-radio evidence where physical values matter |

Issue #13 owns all human-supervised measurements and measurement-dependent circuit
choices. Architecture tests may use fixtures and explicit unknown parameters; they
must not invent KX2/KX3 voltage, impedance, bias, threshold, ground, insertion, or
timing results.

## M1 development-platform binding

ADR 0007 selects a Nordic nRF5340 DK plus a replaceable external Pmod I2S2 only for the
instrumentable M1 probe. The network core runs the BLE controller; the application
core owns USB, the external-I²S audio service and conversion, protocol services, typed
CAT, diagnostics, monotonic lease timing, watchdog integration, and the sole PTT
safety service. This allocation does not prove simultaneous operation or remove the
application core as a common-mode fault.

The selected resources and staged probe must demonstrate:

- simultaneous class-compliant full-duplex USB Audio device operation and BLE
  peripheral/GATT server operation with independently observable and injectable
  failures;
- a dedicated firmware-monotonic lease timer and separately configured watchdog
  capable of the accepted safety bounds;
- an observable external-I²S conversion interface with deterministic mute, format
  conversion, USB-disciplined clock/buffer health, and digital/analog loopback;
- separate CAT EasyDMA and test access for one normally open PTT command, physical TX
  inhibit, downstream radio-side output sensing, profile/harness state, and fault
  instrumentation;
- passive receive-safe behavior through reset, brownout, bootloader, and update; and
- externally powered and direct-phone bench paths using documented limits, with no
  assumption that an iPhone powers or accepts the interface.

ADR 0007 fixes development descriptors, conversion, buffering, resource ceilings,
power experiments, BOM, and fallback triggers in the linked specification. Issues
#12, #13, and #18 must preserve its evidence labels: documented capability is not
measured operation.

Still deferred are production components, Rev A USB topology and codec, radio-side
analog and switch/sense parts, PCB, enclosure, harness identification technology,
security policy beyond the bounded local M1 contract, stable public SDK, Android
application, and additional radios.

The USB Audio plus USB MIDI alternative remains conditional on ADR 0003's validation
trigger. The primary architecture does not include MIDI. If #18 supplies triggering
evidence, a transport adapter may carry the same host-neutral v0 payload semantics
only after an explicit ADR update.
