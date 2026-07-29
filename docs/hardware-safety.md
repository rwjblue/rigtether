# Hardware and transmit safety

RigTether controls a radio transmitter. Safety behavior is part of the product
contract, not a later hardening task. [ADR 0004] defines the feasibility baseline.

This document separates four kinds of statement:

- **Documented fact** comes from a cited manufacturer, platform, or transport source.
- **Engineering decision** is a required RigTether behavior chosen for the feasibility
  baseline.
- **Engineering inference** is a design conclusion that still needs implementation or
  bench evidence.
- **Physical result** is a measured observation. There are no physical results in this
  baseline.

## Safety state and scope

The **receive-safe state** means all RigTether-controlled hardware PTT outputs are
electrically inactive, the transmit-audio path is muted or quiescent, no RigTether CAT
operation can request transmit, and no transmit lease is valid. It does not claim that
the radio cannot transmit from its front panel, a microphone, VOX, an internal keyer,
another accessory, or an internal fault.

For M1, the radio must be configured with VOX and other automatic keying disabled.
Hardware PTT is the only RigTether-controlled transmit mechanism. CAT commands that
enter or sustain transmit are not part of the raw or typed CAT surface. A future
additional keying mechanism requires a new safety decision that defines arbitration
with the hardware PTT path.

The interface has three safety states:

| State | Output and lease behavior | Entry and exit |
| --- | --- | --- |
| `RECEIVE_SAFE` | PTT inactive; transmit audio muted; no lease | Default on every boot, reset, disconnect, update, profile change, expiry, or fault. A valid acquire may enter `TX_ACTIVE`. |
| `TX_ACTIVE` | Exactly one current session owns one bounded lease; PTT may be active only while every precondition remains true | Renewal may extend the deadline but never the continuous-TX cap. Release, timeout, loss of a precondition, or fault returns to `RECEIVE_SAFE` or enters `FAULT_LOCKOUT`. |
| `FAULT_LOCKOUT` | PTT inactive when the output path remains controllable; new acquire and renewal are rejected | Requires the fault to clear, actual output to be observed inactive, and an explicit reset/recovery action. Power cycling alone must not restore transmit intent. |

An output stuck active is a residual hardware fault: firmware cannot make that node
receive-safe. The independent physical TX inhibit is the required recovery boundary.

## Transmit ownership and lease contract

These semantics are protocol requirements. The
[v0 control contract](../protocol/README.md) owns their wire representation, not their
meaning.

### Identity and authority

- Firmware is the sole authority that may energize the hardware PTT output.
- The owner is the tuple `(boot_id, session_id, lease_id)`. `boot_id` changes on every
  firmware boot. `session_id` is newly negotiated for each control session.
  `lease_id` is unpredictable or monotonically unique within that session.
- Only one lease may exist. A new session atomically releases and invalidates every
  older lease before it can become active.
- USB audio presence, audio samples, CAT traffic, BLE connection state, cached app
  state, and a restored platform connection are never transmit authority.
- The host may acquire or renew only while it has current, explicit operator transmit
  intent. It must stop renewing on intent release, app lifecycle loss, route loss,
  control error, or uncertainty. The device guarantee does not depend on the host
  doing so correctly.

### Timing

| Symbol | Feasibility value | Required behavior |
| --- | --- | --- |
| `T_LEASE_MAX` | 500 ms | Maximum interval from firmware acceptance of acquire or renewal to expiry. The device may grant less, never more. |
| `T_RENEW_TARGET` | 250 ms | Host target between accepted renewal attempts. This is not a grace period or a device guarantee. |
| `T_RELEASE_MAX` | 100 ms | Maximum from expiry or a detected safety event to electrical PTT release, when the output path remains controllable. |
| `T_CONTINUOUS_MAX` | 60 s | Non-extendable maximum from first PTT assertion in one operator-intent epoch. |
| `T_REARM_MIN` | 1 s | Minimum continuously observed receive-safe interval after the continuous cap before a new operator-intent epoch may acquire. |
| `T_WATCHDOG_RELEASE_MAX` | 500 ms | Maximum from loss of the firmware safety-service heartbeat to watchdog reset and electrical release. |

All device deadlines use a firmware-owned monotonic clock. Wall-clock changes, host
timestamps, BLE supervision timeouts, and application scheduling cannot extend them.
Crossing a deadline has no grace period. Firmware must schedule the inactive output
before non-safety work and meet `T_RELEASE_MAX`.

`T_CONTINUOUS_MAX` prevents one stuck renewal loop from maintaining an indefinite
transmission. Reaching it releases PTT, enters `FAULT_LOCKOUT`, and rejects acquire or
renew until the host has reported intent released, the output has remained inactive
for `T_REARM_MIN`, and a new operator action begins a new intent epoch. A protocol
reconnect or power cycle does not count as that operator action.

### Commands, renewal, and replay

1. Acquire is accepted only in `RECEIVE_SAFE`, for the current session, with TX inhibit
   not active, actual output observed inactive, required audio/control health valid,
   the radio profile validated, and no latched fault.
2. Firmware chooses a duration no greater than `T_LEASE_MAX`, creates a new `lease_id`,
   and reports the device-relative expiry before or with actual-output status.
3. A renewal is accepted only for the exact owner tuple, an unexpired lease, and the
   next operation sequence. Its deadline is `accept_time + granted_duration`; renewal
   never adds time to the previous deadline and never changes the continuous cap.
4. An exact duplicate operation returns the cached result and causes no state change.
   In particular, a duplicate acquire or renewal does not extend a deadline.
5. An altered reuse of an operation identifier, an out-of-order operation, malformed
   input, or a current-channel command with the wrong session is a protocol fault. It
   cannot assert PTT and releases any active lease into `FAULT_LOCKOUT`.
6. A renewal received at or after expiry is rejected. It cannot revive a lease. A new
   operator action and new acquire are required after the receive-safe and rearm
   conditions are met.
7. Release is idempotent. Firmware first requests the output inactive, invalidates the
   lease, then acknowledges with actual-output status. Missing acknowledgement cannot
   keep the lease alive.

## Enforcement boundaries

| Guarantee | Hardware | Firmware | Host software |
| --- | --- | --- | --- |
| Receive on no power, reset, and unconfigured pins | Normally open PTT switch; inactive passive bias; no stored energy may hold assertion | Configure inactive output before other peripherals and throughout boot/update | Treat every connection as receive-safe until status proves otherwise |
| Bounded transmit after host or link failure | Cannot depend on BLE; watchdog reset must expose the inactive hardware bias | Monotonic lease timer, continuous cap, independent watchdog, and immediate release path | Renew by `T_RENEW_TARGET`; stop on intent/lifecycle/route/control uncertainty |
| One owner; no stale reassertion | No hardware memory of transmit intent | Fresh boot/session/lease identities; sequence and duplicate rules; no persisted lease | Never persist/restore transmit intent; discard cached operations on reconnect |
| Independent operator inhibit | Series physical control downstream of the controllable PTT switch; open must prevent assertion | Sense and report inhibit where practical; deny acquire while inhibited | Show inhibit state and do not offer transmit as available |
| Actual-output visibility | Sense the radio-side PTT node downstream of the output switch and inhibit; local indication derives from that sensed state | Compare requested and sensed state, publish both, and latch disagreement | Display sensed output as `PTT output`, never command state as `transmitting` |
| CAT and audio cannot bypass PTT | Keep paths electrically distinct until measurement justifies combining them | Reject CAT transmit operations; audio samples never assert PTT; audio fault releases lease | Do not synthesize PTT from audio and do not expose unsafe raw CAT commands |
| Fault containment | Current limiting, protection, test points, and physical disconnect remain required | Any detected safety fault releases, mutes, records reason, and locks out | Surface the fault; never retry transmit automatically |

Per [ADR 0005], the firmware watchdog must be driven by the dedicated, high-priority
safety service only after it has checked
the lease deadline, inhibit input, output feedback, control session, profile state, and
required audio health. A timer interrupt that only proves the MCU is clocking is not a
sufficient heartbeat.

## TX inhibit and indication recommendations

### M1 bench

- Fit an accessible, clearly labeled, normally open series TX-inhibit switch or
  removable link between the controllable PTT switch and the radio-side PTT conductor.
  Opening it must prevent assertion despite MCU, firmware, host, or control-link state.
- Put a test point on each side of the inhibit. The output-side point is the acceptance
  reference for actual PTT output.
- Sense the output-side node with sufficiently high impedance that the sense circuit
  cannot key the radio or defeat the inactive bias. Drive the local `PTT OUT` indicator
  from that sensed state, not from the command or MCU drive signal.
- Expose commanded state, sensed output state, inhibit state when available, owner
  identity, deadline remaining, continuous elapsed time, and last release/fault reason
  in firmware diagnostics.
- Label this indication `PTT OUT`, not `RF`, `ON AIR`, or `RADIO TX`. It proves only
  the interface output. The radio can reject PTT or transmit for another reason.

Use a radio-disconnected electrical fixture for automated checks. Human validation
with a KX2 or KX3 belongs to issue #13 and must use the documented dummy-load and
minimum-practical-power precautions if the radio is allowed to generate RF.

### Rev A

Preserve the independent series inhibit, radio-side output sensing, local indication,
diagnostic status, current limiting, ESD protection, and test access. The implementation
may change form, but deleting one requires a new safety decision with equivalent
independent control and observability. A true RF-output detector or radio telemetry may
be evaluated later; neither is implied by `PTT OUT`.

The KX3 documents an ACC2 GPIO mode that can inhibit transmit. It is a possible
radio-specific second layer, not the cross-radio RigTether inhibit baseline. The KX2
manual documents mic-jack PTT but no equivalent general-purpose inhibit in the cited
material. The [KX2/KX3 interface specification] fixes the documented connector and
configuration contract while leaving unpublished electrical values for human
measurement. KX3 `ACC2 IO LO=PTT` is compatible with the normally open sink baseline;
`HI=PTT` is not. [ADR 0005] allocates exactly one selectable PTT path per profile
without inventing missing electrical values; the measured bench choice follows in
issue #13. Neither may use the native inhibit as a substitute for the independent
series inhibit.

## Hazard and failure-mode analysis

All timing bounds below assume the PTT output path remains controllable. `last renewal`
means the last renewal accepted by firmware, not sent by the host.

| Hazard or failure | Required response and bound | Primary enforcement | Residual or unresolved risk |
| --- | --- | --- | --- |
| Power-on, brownout, or power removal | Remain or become inactive by passive hardware state; no restored lease | Hardware bias; firmware boot order | Brownout behavior and stored-energy release require schematic review and measurement |
| MCU reset, crash, assertion, or watchdog expiry | PTT inactive by `T_WATCHDOG_RELEASE_MAX`; reboot in `RECEIVE_SAFE` with new `boot_id` | Hardware reset state; firmware watchdog | A shorted output device is outside firmware control |
| USB detach, route loss, or required transmit-audio fault | Release within `T_RELEASE_MAX` after detection and never later than lease expiry plus `T_RELEASE_MAX`; mute audio | Firmware health state and lease | Exact route-loss detection latency is an M1 measurement |
| BLE/control loss or radio interference | No renewal means release no later than 600 ms after the last accepted renewal | Firmware lease | Bluetooth supervision loss may be slower and is not the bound |
| App suspension, termination, crash, or force quit | Host stops when able; device releases within the lease bound without requiring a callback | Firmware lease; host defense in depth | Platform may not deliver a final lifecycle callback |
| Stale, delayed, wrong-session, or post-reconnect command | Never assert or revive PTT; current-channel identity/ordering fault releases and locks out | Firmware identities and sequencing | The v0 contract fixes strict JSON, ordered identities, and machine-readable vectors |
| Exact duplicate command | Return cached result; do not extend a deadline or repeat a transition | Firmware idempotency cache | The v0 contract fixes 128-bit operation identities and a non-evicting session cache |
| Stuck host renewal loop | Hard release at 60 s plus 100 ms; lock out until release intent, 1 s safe interval, and new operator action | Firmware continuous cap | Firmware cannot prove a host event was genuinely human; physical inhibit remains final authority |
| Radio-profile change or harness identity change | Release and mute first; invalidate session and lease; validate new profile before acquire | Firmware state machine | Harness identity mechanism is not yet selected |
| Firmware-update request, bootloader entry, or failed update | Reject update while output is sensed active; otherwise enter update with inhibit active and PTT passively inactive | Hardware bias; bootloader and firmware | Bootloader and rollback behavior need implementation evidence |
| CAT timeout, parse error, or unexpected response | Release and lock out because radio configuration/state is uncertain; PTT release cannot wait for CAT | Firmware PTT path independent of CAT | Radio may remain in TX if keyed independently of RigTether |
| Unsafe raw CAT transmit command | Reject before radio I/O; no lease or output change | Protocol and radio-profile allowlist | Programmer-reference command inventory must be maintained |
| Audio samples, silence, clipping, underrun, or stream start | Never assert PTT; a declared transmit-audio health fault releases an active lease | Hardware/firmware separation and [ADR 0005] ownership | ADR 0007 fixes initial `audio healthy` thresholds; #12/#18 validate detection and timing |
| Controllable output commanded active but sensed inactive | Release command, mute, latch fault, and report mismatch; do not retry | Output sensing and firmware | Could be open harness, inhibit, failed switch, or sensor; diagnosis needs bench evidence |
| Controllable output commanded inactive but sensed active | Reassert inactive, mute, latch fault, report urgently; operator opens physical inhibit | Output sensing; physical inhibit | A shorted PTT device or conductor can sustain transmit until physically interrupted |
| Output sensor stuck or misleading | Sensor must not create authority; cross-check at test point during validation | Hardware independence; tests | A false inactive reading can hide a stuck output; single-fault diagnostic coverage is not yet proven |
| TX-inhibit contact open | Assertion is physically impossible; report inhibited if sensed | Hardware | Open is intentionally safe |
| TX-inhibit shorted or bypassed | Normal lease protections remain; inspection/test must detect loss of independent inhibit | Hardware checkout | Combined inhibit bypass and PTT short is a residual multiple fault |
| PTT conductor shorted to its active state or wrong harness wiring | Firmware may detect mismatch but cannot guarantee release; use physical inhibit/disconnect and lock out | Physical inhibit, protection, harness checkout | Electrical levels and connector fault effects await #3 and #13 |
| Radio front-panel, microphone, VOX, keyer, or other accessory transmits | RigTether releases its own output and reports only what it can observe | Operator setup and radio behavior | RigTether cannot guarantee whole-radio receive state against independent key sources |

## Objectively testable acceptance checks

The M1 fixture must represent the documented radio PTT input without an antenna or RF
generation. The [KX2/KX3 interface specification] supplies the published limits and
explicitly labels all unpublished voltage, current, impedance, ground, and timing
values as measurement-required. Do not substitute typical values for those unknowns.

| Check | Stimulus | Pass criterion |
| --- | --- | --- |
| Default electrical state | Power interface off; power on; hold MCU in reset; release reset | Radio-side PTT test point remains inactive throughout, except any transition already bounded and approved from schematic analysis |
| Brownout and repeated reset | Sweep or interrupt interface supply with a fixture; trigger software reset repeatedly | No PTT assertion; each boot has a new `boot_id` and no lease |
| Lease expiry | Acquire the maximum lease and send no renewal | PTT releases no later than 600 ms after acquire acceptance; status records `lease_expired` |
| Renewal | Renew at 250 ms while explicit intent remains | Each accepted renewal sets, rather than adds to, a deadline no more than 500 ms ahead |
| Continuous cap | Renew continuously beyond 60 s | PTT releases by 60.1 s from first assertion, enters lockout, and remains inactive despite further renew/acquire traffic |
| Rearm | After continuous-cap lockout, try reconnect, power cycle, renew, and acquire; then report release intent, observe 1 s inactive, and provide a new intent action | Only the final complete rearm sequence can create a new lease |
| Duplicate and replay | Repeat identical acquire/renew/release packets; replay older and altered-duplicate packets | Exact duplicates cause no transition or extension; stale/altered operations never assert and current-channel faults release/lock out |
| Session replacement | Acquire, disconnect, reconnect, restore cached host state, and replay old traffic | Old output releases within the lease bound; new session starts safe; old tuple never asserts |
| Host lifecycle | With an active lease, suspend, terminate, crash, and force quit the test app separately | Output releases within the lease bound without relying on a lifecycle callback; restart does not restore intent |
| BLE/control loss | With an active lease, disable the central, block traffic, and force supervision timeout | Output releases within 600 ms of the last accepted renewal regardless of disconnect callback timing |
| USB/audio loss | With an active lease, detach USB and separately force route loss/stream fault | Firmware releases within 100 ms of detecting the fault and no later than the lease bound; audio becomes muted |
| Firmware failure and update | Stall the safety heartbeat; trigger watchdog; request update while inactive and while active; interrupt update | Watchdog releases within 500 ms; active update is rejected or releases first; bootloader/update states never assert |
| Profile and CAT faults | Change profile; inject CAT timeout, malformed response, and forbidden transmit command | Active lease releases and locks out; forbidden command never reaches radio I/O |
| Inhibit | Close then open TX inhibit while assertion is requested; attempt acquire with inhibit open | Output-side PTT is inactive with inhibit open independent of software; acquire is denied |
| Actual-output indication | Exercise inactive, active, inhibited, open-output, and forced-active fixture states | Local indication and reported sensed state follow the output-side test point, not commanded state; mismatch latches a fault |
| Stuck-active fault | Force the fixture's radio-side PTT active after firmware requests inactive | Firmware detects and reports mismatch; physical inhibit opening makes the output-side node inactive |

Record scope traces or timestamped logic captures, fixture schematic and revision,
instrument models, firmware/app revisions, configured bounds, and observed worst cases.
A checklist pass is not a claim that a real KX2 or KX3 behaved the same way.

## Evidence and source classification

Sources were accessed 2026-07-29.

| Classification | Material fact or conclusion | Source |
| --- | --- | --- |
| Documented fact | The KX2 mic connector includes a PTT contact; Elecraft says an always-on TX LED can indicate external equipment holding PTT. | [Elecraft KX2 Owner's Manual, Rev B2](https://ftp.elecraft.com/KX2/Manuals%20Downloads/KX2%20owner%27s%20man%20B2.pdf) |
| Documented fact | KX2 mic PTT is ground-active. KX3 mic PTT is ground-active, while KX3 ACC2 GPIO documents low-active PTT and optional low/high-active inhibit modes. | [KX2/KX3 interface specification](elecraft-kx2-kx3-interface.md) and its revisioned Elecraft sources |
| Documented fact | KX3 can be remotely powered by 8 to 12 V on its mic-PTT conductor for at least 100 ms; RigTether must never source that conductor. | [Elecraft KX3 Owner's Manual, Rev C5](https://ftp.elecraft.com/KX3/Manuals%20Downloads/E740163%20KX3%20Owner%27s%20man%20Rev%20C5.pdf) |
| Documented fact | iOS normally suspends background apps; Core Bluetooth background modes provide event-oriented execution but do not run forever, and restoration does not apply in every user/device state. | [Apple background execution modes](https://developer.apple.com/documentation/xcode/configuring-background-execution-modes), [Core Bluetooth background processing](https://developer.apple.com/library/archive/documentation/NetworkingInternetWeb/Conceptual/CoreBluetooth_concepts/CoreBluetoothBackgroundProcessingForIOSApps/PerformingTasksWhileYourAppIsInTheBackground.html), [TN3115 Bluetooth restoration rules](https://developer.apple.com/documentation/technotes/tn3115-bluetooth-state-restoration-app-relaunch-rules) |
| Documented fact | Bluetooth LE connection supervision detects link loss, but its timeout is negotiated over a broad range rather than being a RigTether application deadline. | [Bluetooth Core 6.2, Link Layer §4.5.2](https://www.bluetooth.com/wp-content/uploads/Files/Specification/HTML/Core-62/out/en/low-energy-controller/link-layer-specification.html#UUID-702f1490-4894-c9bc-422b-05e67e88f7e4) |
| Engineering inference | A device-owned short lease is required because neither app lifecycle callbacks nor BLE disconnect detection gives the required release bound. | Derived from the cited Apple and Bluetooth behavior; verified only when the M1 timing checks pass |
| Engineering decision | The numeric bounds, single hardware-PTT owner, CAT-keying exclusion, continuous cap, physical inhibit, and sensed-output requirements are the RigTether feasibility baseline. | [ADR 0004](decisions/0004-bound-transmit-authority-with-device-enforced-leases.md) |
| Physical result | None. No circuit was built, radio keyed, or timing measured for this decision. | Human bench evidence is deferred to #13 and related M1 validation |

## Unknowns and required follow-up

- The [KX2/KX3 interface specification] establishes documented pins, active levels,
  published limits, required settings, CAT allowlist, hazards, and measurement plans.
  Bias, impedance, thresholds, ground relationships, and insertion behavior not
  published by Elecraft remain explicitly unmeasured and human-required.
- [ADR 0005] assigns independent control, host-route, device-media, CAT/profile,
  inhibit, and output health to explicit owners. [ADR 0007] and the
  [M1 development-platform specification](m1-development-platform.md) select initial
  audio-health thresholds. Issue #12 must implement and software-test the
  fault-to-safety-service path; #18 must measure media/clock detection under
  simultaneous current-device transport; and #13 must measure the hardware release
  path within ADR 0004's bounds.
- The [v0 control contract](../protocol/README.md) defines strict JSON encoding,
  runtime GATT fragmentation, 128-bit identities, session establishment,
  acknowledgement, errors, status fields, and
  [conformance vectors](../protocol/vectors/v0.json) without changing these semantics.
- Issue #13 must validate inactive bias, reset/brownout behavior, watchdog release,
  lease timing, continuous cap, output sensing, inhibit independence, loading,
  protection, and the stuck-active fixture fault.
- ADR 0007 selects an nRF5340 DK and nominal 250 ms application watchdog only for the
  replaceable M1 probe. Production MCU/watchdog, output switch, sense circuit, inhibit
  implementation, protection, and connectors remain unselected. No component is
  physically qualified by this analysis.
- Actual radio behavior, RF output, app/device timing, and acceptable operational
  ergonomics remain physical results to measure. A proposed change to a safety bound
  requires evidence and an explicit ADR update.

## Bench rules

- Prefer the radio-disconnected fixture for every automated safety check.
- Any later radio-connected transmit validation requires a suitable dummy load, human
  supervision, minimum practical RF power, and the radio manufacturer's procedures.
- Keep the physical TX inhibit and a means to remove radio power within reach.
- Record setup, instruments, wiring, hardware/firmware/app revisions, expected bounds,
  observed bounds, and photographs or diagrams where useful.
- Stop on unexpected heating, RF output, current draw, ALC, distortion, or latched PTT.

This document is an engineering policy and feasibility decision, not a safety
certification, regulatory conclusion, or replacement for the radio operating manual.

[ADR 0004]: decisions/0004-bound-transmit-authority-with-device-enforced-leases.md
[ADR 0005]: decisions/0005-separate-host-media-control-radio-and-safety-boundaries.md
[KX2/KX3 interface specification]: elecraft-kx2-kx3-interface.md
