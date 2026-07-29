# Elecraft KX2/KX3 radio interface specification

This document is the source-backed M0 contract for the replaceable KX2 and KX3 radio
profiles. It covers receive audio, transmit audio, CAT, and PTT. It is not a released
harness design or a claim that either radio has been physically tested.

The classifications used below are:

- **Documented fact:** stated in a cited Elecraft document, including a numerical
  limit only when Elecraft publishes one.
- **Engineering implication:** a RigTether design or validation consequence derived
  from documented facts.
- **Unresolved:** behavior that depends on firmware, configuration, an implementation
  choice, or documentation that does not publish the required detail.
- **Measurement required:** a value or behavior that must be established on a real
  radio with the human-supervised procedure in this document.
- **Physical result:** an observation from a real setup. There are no physical results
  in this specification.

## Source baseline

All sources were accessed 2026-07-29. Revision dates in this table are Elecraft's
published document or index dates, not measurement dates.

| Source | Revision or published date | Applicable evidence |
| --- | --- | --- |
| [KX2 Owner's Manual](https://ftp.elecraft.com/KX2/Manuals%20Downloads/KX2%20owner%27s%20man%20B2.pdf) | Rev B2, 2023-03-22 | KX2 connectors, menu settings, audio output, ACC signaling, PTT behavior, specifications |
| [KX2 schematic set](https://ftp.elecraft.com/KX2/Manuals%20Downloads/E740324%20KX2%20Schematic%20Files%20RevA.pdf) | Rev A, 2017-07-28 | Connector nets and visible series components; not a substitute for port limits |
| [KX3 Owner's Manual](https://ftp.elecraft.com/KX3/Manuals%20Downloads/E740163%20KX3%20Owner%27s%20man%20Rev%20C5.pdf) | Rev C5, 2014-06-16 | KX3 connectors, menu settings, audio output, ACC1/ACC2 behavior, PTT behavior, specifications |
| [KX3 Owner's Manual errata](https://ftp.elecraft.com/KX3/Manuals%20Downloads/KX3%20Owner%27sManErrata%20C5-4.pdf) | Rev C5-4, 2026-03-05 | Current corrections checked; none changes the interfaces specified here |
| [KX3 schematic set](https://ftp.elecraft.com/KX3/Manuals%20Downloads/KX3SchematicDiagramDec2012.pdf) | March 2013 package | Connector nets and visible series components; not a substitute for port limits |
| [K3/KX3/KX2 Programmer's Reference](https://ftp.elecraft.com/KX2/Manuals%20Downloads/K3S%26K3%26KX3%26KX2%20Pgmrs%20Ref%2C%20G5.pdf) | Rev G5, 2019-02-20 | Command framing, timing, baud-rate choices, command and response formats |
| [KX3-PCKT cable-set instructions](https://ftp.elecraft.com/KX3/Manuals%20Downloads/E740169%20KX3-PCKT%20Cable%20Set%20Instructions%20Rev%200.pdf) | Rev 0, 2012-08-20 | Manufacturer cable routing for KX3 audio, ACC1, and ACC2 |
| [MH3 microphone product documentation](https://elecraft.com/products/mh3-kx-line) | Current product page | Electret microphone, required bias, 3.5 mm four-conductor plug, KX2/KX3 applicability |
| [KXUSB cable product documentation](https://elecraft.com/products/kxusba-extra-cable-usb-rs232-to-3-5mm-cable) | Current product page | Supported-radio list only; no circuit or radio-port electrical limits are published |
| [KX2 manuals index](https://elecraft.com/pages/kx2-fully-portable-transceiver-manuals) and [KX3 manuals index](https://elecraft.com/pages/kx3-high-perofrmance-portable-transceiver-manuals) | Current indexes | Confirmation that the cited owner manuals, programmer reference, schematics, and errata are the current published set |

The shared programmer reference's support for both radios does not establish electrical
or behavioral equivalence. Each profile below retains its own connector, menu, command,
firmware, and measurement evidence.

## Connector summary

Contact names use the owner's-manual view of a fully inserted plug. Numbering or wire
colors are deliberately omitted because neither is a safe substitute for a continuity
check of the actual harness.

| Function | KX2 | KX3 |
| --- | --- | --- |
| Receive audio | `PHONES`, 3.5 mm TRS: tip left, ring right, sleeve return | `PHONES`, 3.5 mm TRS: tip left, ring right, sleeve return |
| Transmit audio and mic PTT | `MIC`, 3.5 mm TRRS: tip mic audio, ring 1 PTT/UP/DN, ring 2 logic ground, sleeve shield ground | `MIC`, 3.5 mm TRRS: tip mic audio, ring 1 PTT/UP/DN, ring 2 logic ground, sleeve shield ground |
| CAT | `ACC`, 3.5 mm TRRS: tip RX Data/radio input, ring 1 TX Data/radio output, ring 2 Key Out, sleeve ground | `ACC1`, 3.5 mm TRS: tip RX Data/radio input, ring TX Data/radio output, sleeve ground |
| Alternate PTT or inhibit | No equivalent general-purpose input is documented | `ACC2`, 2.5 mm TRS: tip GPIO, ring Keyline output, sleeve ground |

Directions are relative to the radio. `RX Data` is therefore data received by the
radio, not audio received over the air.

## Applicable radio state and settings

These are the safe characterization defaults. A support profile may deviate only after
recording the reason and evidence. Menu state is firmware-dependent and must be read
from or checked on the actual radio rather than inferred from a factory default.

| Concern | KX2 | KX3 |
| --- | --- | --- |
| Microphone bias | `MIC BIAS` is measurement-dependent; start characterization with `OFF`, then test `ON` separately | Same |
| Mic buttons/PTT | `MIC BTN PTT`; do not enable unused UP/DN decoding | Same |
| Automatic audio/keyer transmit | `VOX MD OFF` in the selected mode | Select PTT rather than VOX with the `VOX` switch; confirm the VOX icon is off |
| Audio characterization | `TX GATE OFF`, speech compression 0, TX EQ flat, ALC on; record mode and mic gain | Same |
| CAT | `RS232 4800 b`, `AUTO INF NOR`; session sends `AI0`, `K20`, and `K30` and verifies each | Same |
| GPIO | Not applicable | `ACC2 IO OFF` unless the selected single PTT path is `LO=PTT`; native inhibit is a separate optional test |

KX2 `PSK-D` and `FSK-D` are not valid M1 transmit-profile modes: its manual says VOX
is always on and PTT is unavailable in those modes. More generally, firmware must
deny a lease and keep transmit audio muted unless the current model, mode, submode,
split state, VOX state, and selected PTT path match a measured profile. `MD`, `IF`, and
front-panel setup provide defensive checks; they do not transfer PTT authority to CAT.

## Receive audio

### Documented facts

- Both radios accept mono or stereo plugs at `PHONES`. A stereo plug can carry left
  and right audio; the owner manuals describe headphones or externally amplified
  speakers as loads.
- Both owner manuals specify 0.1 W per channel stereo output. This is an available
  output-power statement, not an impedance, DC offset, or absolute voltage limit.
- The published schematics show a 10.0 ohm series component in each left and right
  output. That component alone does not establish total source impedance over
  frequency or a permitted load.
- The KX2 internal speaker specification is 0.3 W and the KX3 internal speaker
  specification is 0.5 W. Those figures do not apply to the external `PHONES` port.

### Engineering implications

- RigTether must receive both channels or explicitly choose and document a
  radio-profile downmix. It must not short left and right outputs together.
- The audio input must begin high impedance and provide DC blocking, bounded
  attenuation, and protection before a codec or amplifier is selected.
- A 3.5 mm connector that fits is not evidence that a line-level input is safe.
  Gain, clipping margin, channel mapping, and mono-plug behavior require validation.

### Unresolved and measurement-required values

The manuals do not publish the recommended or minimum external load, full source
impedance, DC common-mode voltage, maximum unloaded or loaded RMS voltage, clipping
point versus AF gain, channel correlation, or transient behavior during insertion.
Measure each value independently on KX2 and KX3. Do not derive an absolute voltage
limit from the 0.1 W rating or the schematic's 10.0 ohm components.

## Transmit audio and microphone bias

### Documented facts

- Both `MIC` ports use the same documented contact functions: mic audio on tip,
  PTT/UP/DN on ring 1, logic ground on ring 2, and shield ground on sleeve.
- The MH3 is an electret microphone and requires `MIC BIAS` enabled. Both owner manuals
  direct MH3 users to `MIC BIAS ON` and `MIC BTN PTT UP.DN`.
- Other microphones can be configured through the menu. Neither owner manual gives a
  RigTether-compatible line-input voltage, input impedance, bias voltage, bias source
  impedance, or absolute input limit.
- The published KX2 and KX3 connector schematics each show a 220 ohm series component
  on the mic-audio conductor and another on the PTT/button conductor. These visible
  components are not complete port specifications.
- Both owner manuals use an ALC display for microphone-level adjustment. That is an
  operational indication, not a calibrated input-voltage specification.
- Both owner manuals describe low-level computer audio as a possible mic input. The
  KX3-PCKT instructions route a computer line output to KX3 `MIC` and KX3 `PHONES` to
  a computer mic/line input. Neither statement supplies an electrical level or limit,
  and the KX3 owner manual warns that an attenuator may be needed.

### Engineering implications

- RigTether transmit audio needs DC blocking and bounded gain/attenuation. It must not
  sink microphone bias until the measured bias network and the chosen coupling
  topology show that to be safe.
- `MIC BIAS OFF` is the conservative configuration candidate for an actively driven,
  AC-coupled RigTether source, but it is not yet a final setting.
  [ADR 0005](decisions/0005-separate-host-media-control-radio-and-safety-boundaries.md)
  allocates a configurable radio-profile boundary for bias and coupling without
  selecting a value. Issue #13 selects and records the bench setting and topology from
  measurement evidence.
- Shield ground and logic ground remain distinct harness requirements. A schematic net
  relationship does not authorize combining them in the interface, cable, or PCB
  before continuity, powered differential-voltage, and ground-current tests.
- VOX is outside M1 transmit authority. Use the model-specific PTT settings in the
  settings table; audio alone must never key either radio.

### Unresolved and measurement-required values

For each radio and for both `MIC BIAS OFF` and `ON`, measure bias voltage, effective
source resistance, input impedance over the intended voice band, DC common mode,
low-level gain, signal amplitude at the intended ALC indication, clipping onset, and
interaction with PTT/button decoding. A physical result is required before issue #13
may fix the bench codec range, attenuator, coupling capacitor, or final menu settings.

## Hardware PTT

### Documented facts

- On both radios, grounding the `MIC` PTT/button conductor can activate PTT. The owner
  manuals' `ERR PTT` troubleshooting text identifies external equipment shorting a
  PTT or key line to ground as a cause of transmit.
- On KX3 only, `ACC2 IO` can configure the GPIO as `LO=PTT`, where a low/0 V input
  activates PTT, or `HI=PTT`, where 3 to 5 V activates PTT. The KX3 specification
  describes the GPIO as 3 V logic I/O, 0 to 5.5 V input tolerant, with a 500 ohm
  series current-limiting resistor. The owner manual says an external circuit may be
  required.
- KX3 `ACC2 IO` can instead be `LO=INH` or `HI=INH`, using respectively 0 V or 3 to
  5 V to inhibit transmit. The manual warns that a floating input in `HI=INH` may be
  interpreted as high and prevent transmission.
- KX3 can be remotely powered on by applying 8 to 12 V DC to the `MIC` PTT conductor
  for at least 100 ms. PTT at `ACC2` does not provide that power-on function.
- KX2 `ACC` Key Out and KX3 `ACC2` Keyline are radio outputs that go low during
  transmit. Each has a documented 30 V, 100 mA maximum. They are not PTT inputs and
  must not be driven by RigTether.
- The KX2 owner manual does not document a general-purpose inhibit input equivalent
  to KX3 `ACC2 IO`.

### Required RigTether behavior

[ADR 0004](decisions/0004-bound-transmit-authority-with-device-enforced-leases.md)
remains authoritative:

- exactly one selected hardware PTT path is owned solely by firmware;
- the selected path uses a normally open, fail-open sink and is inactive without
  power, during reset, and on every control fault;
- CAT keying, VOX, audio keying, and a second simultaneous PTT path are excluded;
- the 500 ms maximum lease renewal interval, 60 s continuous-transmit cap, independent
  physical series TX inhibit, receive-safe behavior, and radio-side sensed `PTT OUT`
  remain unchanged; and
- Key Out/Keyline or CAT telemetry may supplement diagnostics but cannot substitute
  for sensing the selected PTT node downstream of the independent inhibit.

For KX3, only `ACC2 IO LO=PTT` is compatible with the baseline normally open sink.
`HI=PTT` must not be used. The native inhibit modes may be evaluated as a
radio-specific second layer but cannot replace RigTether's independent physical
series inhibit.

[ADR 0005](decisions/0005-separate-host-media-control-radio-and-safety-boundaries.md)
allocates a single selectable hardware-PTT boundary for each radio profile without
choosing an unmeasured path. Issue #13 compares KX3 `MIC` ring 1 with
`ACC2 IO LO=PTT`, characterizes the KX2 mic-PTT circuit, and records the final bench
choice from measurement evidence. This is an explicit decision boundary: choosing a
path changes the harness and fault analysis. Exactly one path may be populated or
enabled for a profile. Any proposal to drive both, use active-high PTT, omit sensed
`PTT OUT`, or replace the physical inhibit requires an owner decision and a
superseding safety ADR.

RigTether must never source voltage onto KX3 `MIC` PTT. The 8 to 12 V remote-power-on
behavior makes source leakage, charged coupling components, miswiring, and insertion
transients material hazards even when transmit is not intended.

### Unresolved and measurement-required values

The owner manuals do not publish `MIC` PTT open-circuit voltage, activation and release
thresholds, hysteresis, sink current, debounce, maximum permitted sink resistance,
timing, or behavior throughout boot and brownout. KX3 documents the nominal
`ACC2 IO LO=PTT` level but still requires measurement of open voltage, sink current,
threshold, timing, and firmware/menu interaction. Measure the selected path on each
radio and compare KX3 candidates before issue #13 fixes the bench profile.

## CAT electrical interface and configuration

### Documented facts

- KX2 uses `ACC` and KX3 uses `ACC1` with the directions and contacts in the connector
  table. Elecraft identifies KXUSB or KXSER as the applicable computer adapter.
- `RS232` selects 4,800, 9,600, 19,200, or 38,400 bit/s. Both owner manuals use 4,800
  bit/s as the ordinary default. Firmware loading can temporarily change speed and
  then restore it.
- The programmer reference defines semicolon-terminated ASCII commands, normally two
  or three command characters plus optional data. Input is case-insensitive and
  responses use uppercase. Communication is full duplex.
- The programmer reference describes typical response time below 10 ms, possible
  delays of about 100 ms, and up to 500 ms for band changes. `?;` indicates a busy,
  unsupported, or otherwise invalid request in contexts described by the reference.
- A normal 3.5 mm TRS serial plug in KX2 `ACC` shorts the unused Key Out ring 2 to
  ground. The KX2 owner manual explicitly says this is harmless and no current flows.
  That statement applies only to this documented KX2 condition.
- Neither the KXUSB product page nor the cited manuals publishes KXUSB circuitry,
  CAT pin voltage swing, receiver thresholds, idle polarity, source impedance,
  absolute limits, or serial word framing.

### Configuration baseline

Start both profiles with:

| Setting | Baseline | Reason |
| --- | --- | --- |
| `RS232` | `4800` | Documented ordinary default and lowest supported rate |
| `AUTO INF` / `AI` | `NOR` / `AI0` | Prevent unsolicited traffic from confusing a bounded request/response session |
| `K2` command mode | `K20` | Normal command responses |
| `K3` extended mode | `K30` | Stable base response forms for the M1 allowlist |

Record radio model, serial number, firmware revisions, and all relevant menu values
with every transcript. A later rate change needs per-radio electrical and timing
evidence; it is not implied by successful 4,800 bit/s operation.

The baseline must not be described as “RS-232 voltage levels.” `RS232` is Elecraft's
menu name. Until measured, the external electrical signaling and framing remain
unknown. RigTether must not assume KXUSB internals or clone a cable from connector
shape.

### M1 CAT allowlist

The radio adapter accepts typed operations only. It emits the exact allowlisted
commands below, validates the entire response including its terminating semicolon,
and rejects all raw or non-allowlisted CAT before radio I/O.

| Purpose | Command | Required response or verification |
| --- | --- | --- |
| Disable automatic information | `AI0;`, then `AI;` | The set has no required immediate response; the query must return `AI0;` |
| Select normal response mode | `K20;`, then `K2;` | The query must return `K20;` |
| Select base K3 extension mode | `K30;`, then `K3;` | The query must return `K30;` |
| Identify radio/options | `OM;` | Parse the fixed `OM APF---TBXI0n;` field positions from the programmer reference; product code `1` is KX2 and `2` is KX3, with absent option positions represented by `-` |
| Read firmware | `RVM;` | Parse the documented `RVMNN.NN;` main-firmware response; `RVD;` may be queried and recorded when available |
| Read VFO A | `FA;` | Exactly `FA` plus 11 decimal frequency digits and `;`, in Hz |
| Set VFO A | `FAxxxxxxxxxxx;`, then `FA;` | No immediate set response is required; the query must confirm the radio's returned/normalized 11-digit frequency |
| Read operating state | `IF;` | Parse the programmer-reference fixed-width `IF` response, including 11-digit frequency and TX/RX state; reject a malformed or wrong-length response |
| Read operating mode | `MD;` | `MDn;`, where the documented values include 1 LSB, 2 USB, 3 CW, 4 FM where supported, 5 AM, 6 DATA, 7 CW-REV, and 9 DATA-REV |
| Read transmit state | `TQ;` | `TQ0;` for receive or `TQ1;` for transmit/pseudo-transmit |

Before setting `FA`, validate the requested frequency against the selected product
profile and the operator-facing policy. A set is complete only after the query
confirms it. A response timeout, `?;`, unexpected unsolicited data, malformed response,
model mismatch, or inconsistent `IF`/`TQ` state is a control fault. It cannot assert
PTT; during an active lease it triggers ADR 0004 release and lockout without waiting
for CAT recovery.

The implementation and fixtures must account for the documented under-10 ms typical,
approximately 100 ms delayed, and up-to-500 ms band-change cases. Issue #9 owns the
concrete scheduler and timeout values after real transcripts; no timeout may weaken
the independent PTT lease.

`ID;` is not used for product identification because the shared reference returns the
same compatibility identifier. `TX;`, transmit variants, `SWT`/`SWH` actions such as
XMIT or TUNE, `KY`, power-level changes, VOX changes, menu writes, mode writes, baud
writes, and every other command are outside the allowlist. `TQ` and `IF` are
observation only: neither is transmit authority nor a substitute for sensed
`PTT OUT`.

### Unresolved and measurement-required values

Measure CAT idle state, polarity, voltage swing, rise/fall time, source impedance,
receiver threshold region, serial word framing, error behavior, and command/response
timing on each model and firmware revision. Use KXUSB only as an official functional
reference; do not attribute the observed cable-side waveform or circuitry to the
radio port without measuring on the radio side of a passive breakout.

## Grounds and isolation boundary

Treat these as separate interface requirements until measurements justify a bond:

- `MIC` shield ground;
- `MIC` logic ground;
- `PHONES` sleeve/return;
- KX2 `ACC` or KX3 `ACC1` ground;
- KX3 `ACC2` ground; and
- radio chassis and RigTether power/USB shield grounds.

Published schematics are evidence of the documented unit revision, not proof of the
actual harness, contact resistance, powered common-mode voltage, or acceptable loop
current. The radio adapter must preserve independent access and test points until the
continuity and powered measurements below are complete.
[ADR 0005](decisions/0005-separate-host-media-control-radio-and-safety-boundaries.md)
allocates ownership and evidence gates for intentional bonds; issue #13 records each
actual bench bond and its measured fault/loading consequence.

## Connector-insertion and miswiring hazards

Phone plugs make and break contacts sequentially. It is an engineering implication,
not an Elecraft timing specification, that insertion can temporarily bridge audio,
bias, PTT, GPIO, data, and ground contacts.

- KX2's documented harmless TRS-to-Key-Out short must not be generalized to `MIC`,
  `PHONES`, KX3 connectors, or arbitrary insertion states.
- A KX3 `MIC` transient that sources voltage onto ring 1 can invoke the documented
  remote-power-on behavior. A transient to ground can invoke PTT on either radio.
- A KX3 `ACC2` plug used for GPIO PTT also contacts the Keyline output on its ring.
  The harness must isolate that output unless a separately approved high-impedance
  diagnostic input uses it.
- A KX2 serial harness must either use the owner's-manual-approved TRS arrangement or
  leave the Key Out contact electrically isolated. It must never drive Key Out.
- Side load and cable weight must be strain relieved; the KX3 owner manual specifically
  recommends lightweight or right-angle plugs to limit connector stress.

Until model-specific insertion tests are completed, connect and disconnect radio-side
phone plugs only with the radio powered off, RigTether's physical TX inhibit open, PTT
inactive, transmit audio muted, and VOX/automatic keying off.

## Human bench-measurement plan

These are planned measurements, not completed evidence. Every radio-connected test is
`human-required`. Automate first against a radio-disconnected fixture; use a real radio
only for the unknowns that a fixture cannot establish.

The resistance, frequency, baud, and stimulus values in the procedures below are
conservative test starting points or controlled sweep coordinates, not radio-port
ratings. They do not become interface limits unless Elecraft documentation or recorded
physical evidence establishes that conclusion.

### Common preflight and evidence record

Use one KX2 and one KX3 as independent test articles. Record radio model, serial number,
hardware revision if available, main/DSP firmware, complete relevant menu settings,
harness/breakout schematic and revision, RigTether hardware/firmware/app revision,
date, operator, ambient conditions, and every instrument model, serial number,
calibration status, range, bandwidth, probe type, and uncertainty.

Required equipment:

- current-limited radio bench supply within the owner's-manual supply range, plus the
  radio battery state documented;
- isolated, high-input-impedance DMM and oscilloscope probes;
- differential oscilloscope probe or isolated two-channel differential method;
- logic analyzer with a protected high-impedance front end;
- isolated low-distortion audio generator, audio analyzer or true-RMS meter, and
  programmable/high-value resistance loads;
- passive four-conductor breakout fixtures that expose every contact without
  combining grounds;
- current-limited protected open-drain stimulus and resistance decade;
- RF signal generator and attenuation suitable for receiver-only audio tests; and
- a 50 ohm dummy load rated above the radio's configured power, RF power meter or
  detector, and a physically reachable radio power disconnect.

Before any powered radio connection, apply the model-specific PTT/VOX settings above,
mute transmit audio, open the independent TX inhibit, set minimum practical RF power
(begin at the documented `PWR 0.0` setting where supported), remove the antenna, and
attach the suitable dummy load whenever the setup could possibly key. Do not use
on-air transmission. Keep the power disconnect and inhibit within the supervising
human's reach.

Store timestamped raw CSV/data files, CAT byte transcripts, scope/logic screenshots,
photos showing the complete wiring and dummy load, expected-versus-observed tables,
and notes for every anomaly. Never replace a numerical result with “pass.”

Stop immediately on unexpected RF or TX indication, a PTT node that does not release,
instrument overload, radio warning, supply-current excursion beyond the test's
predeclared bound, unexpected voltage outside a cited limit, heating, odor, smoke,
unstable oscillation, or loss of the physical inhibit. Open the inhibit, remove radio
power, preserve captures, and require review before resuming.

### 1. Passive pin and ground map

With radio power and battery removed, use low-test-current four-wire resistance and
continuity measurements through the passive breakout. Confirm every tip/ring/sleeve
mapping and measure each ground-to-ground and ground-to-chassis pair in both plug
orientations. Do not use continuity beeps as numerical evidence.

Then power the radio in receive-safe state with no intentional ground bonds. Use a
high-impedance differential measurement to record DC and AC voltage between every
ground pair. Add one proposed bond at a time through a protected current-measurement
path; record steady and transient current. Stop before bonding if a differential
voltage is outside the planned instrument/protection range or if current exceeds the
predeclared safe fixture bound.

Expected observation: a verified per-unit connectivity and common-mode map, not a
presumption that all symbols named “ground” are interchangeable.

### 2. Receive-audio output

Keep the transmitter physically inhibited. Feed a stable receiver test signal from an
RF generator through appropriate attenuation. Observe each `PHONES` channel first with
at least 100 kilohms differential load, then with descending known loads only after
the previous step is stable and within a manufacturer-supported or reviewed safe
range. Sweep AF gain and representative voice-band frequencies.

Record DC common mode, no-load and loaded RMS voltage, waveform, clipping onset,
left/right correlation, mono/stereo plug behavior, and supply current. Derive
small-signal source impedance only from the measured load response; do not call it an
absolute limit. Stop on clipping outside the expected control range, instability,
warning, or the common stop conditions.

### 3. Microphone bias and transmit-audio input

With PTT inhibited, measure `MIC` tip relative to logic ground and shield using
high-impedance probes for `MIC BIAS OFF` and `ON`. Estimate bias source resistance
using a resistance decade starting at 100 kilohms; do not go below 10 kilohms without
review of the preceding current and voltage data.

For input impedance and gain, inject an isolated, AC-coupled 1 kHz source through a
known large series resistance, beginning in the millivolt range. First use monitor or
receive-safe indications that do not key the radio. If an ALC/transmit observation is
essential, the supervising human may close the inhibit only with dummy load attached,
minimum practical power selected, CAT keying disabled, and the protected hardware PTT
under direct control. Increase level slowly and only far enough to map the intended
ALC range and clipping onset.

Record input impedance versus frequency, bias voltage and source resistance, coupling
transient, amplitude at each ALC indication, distortion/clipping, and interaction with
button decoding. Unexpected TX, full-scale ALC at the initial source level, or RF above
the predeclared minimum-power envelope is an immediate stop.

### 4. PTT candidates and native inhibit

Measure one conductor at a time; never connect or energize both KX3 PTT candidates.
With the inhibit open, record open-circuit voltage and boot/reset waveform using a
high-impedance probe. Use a protected, isolated open-drain stimulus and resistance
decade beginning at 1 megohm, stepping downward only while current remains within the
reviewed fixture bound. Do not inject a positive voltage into `MIC` PTT.

When an activation check is required, attach the dummy load, select minimum practical
power, and have the human close the inhibit for the shortest practical interval.
Record first activation and release resistance, sink current, low voltage, debounce,
latency, boot/brownout behavior, TX indication, RF detector output, and recovery.
Repeat for KX2 `MIC` PTT, KX3 `MIC` PTT, and KX3 `ACC2 IO LO=PTT` as separate tests.
Do not test KX3 `HI=PTT` as a product candidate.

For KX3 native inhibit, test `LO=INH` and `HI=INH` only as optional defense-in-depth,
including the documented floating-input behavior. The independent physical series
inhibit remains installed and is tested separately. Do not inject 8 to 12 V to test
KX3 remote power-on; instead verify with a DMM and scope that an unpowered or resetting
RigTether cannot source the `MIC` PTT conductor.

### 5. CAT electrical and protocol behavior

Use an official KXUSB or KXSER cable only as a functional reference. Insert a passive
breakout at the radio port and use protected high-impedance differential probes so
radio-side and cable-side waveforms remain distinguishable. In receive-safe state at
4,800 bit/s, execute the exact M1 allowlist and capture bytes and both electrical
directions.

Record idle state, polarity, voltage extrema, rise/fall time, bit timing, decoded word
framing, response latency, and every response. Repeat read-only commands at the other
documented rates only if the profile may use them. Estimate output source impedance
with high-value loads; do not seek a destructive limit. Receiver-threshold exploration
requires an isolated, current-limited programmable source, begins inside the observed
waveform, and stops before any undocumented rail or current is approached.

Capture normal, `?;`, delayed, malformed-fixture, disconnect, and recovery cases.
Frequency-setting validation uses a legal receive frequency and confirms with `FA;`;
it does not key the radio. Stop on a warning, unexplained loss of communication that
persists through a documented recovery, unexpected TX, or the common conditions.

### 6. Powered insertion and removal

Perform this hazardous test only after the preceding passive and powered maps have
been reviewed. Keep VOX off, transmit audio muted, minimum power selected, dummy load
attached, and the independent inhibit open. Instrument all contacts and the TX/RF
indications through the breakout. Insert and remove each approved plug using normal
and deliberately slow motion with RigTether outputs disabled.

Record contact order, bridged contacts, transient voltage/current, radio power state,
`TQ`/TX indication, and any RF-detector response. Any PTT assertion, remote power-on,
RF, latched state, overvoltage, or unexplained menu/action event stops the test and
keeps powered insertion prohibited until the harness is changed and reviewed.

## Physical-results register

No human-supervised KX2 or KX3 measurements were supplied or performed for this work.
Every measurement field above is therefore **not yet measured**. Results belong in a
dated, revision-controlled issue #13 evidence artifact; they must not be backfilled
from typical values, another radio model, cable observations, or schematic inference.

## Downstream contract

- [ADR 0005](decisions/0005-separate-host-media-control-radio-and-safety-boundaries.md)
  allocates the adapter, harness, profile, measurement, and safety boundaries without
  filling any unknown with a typical value. It preserves
  separate KX2/KX3 profiles, exactly one selectable hardware PTT path, Key Out
  isolation, and ADR 0004; component and bench-circuit choices that depend on physical
  values remain gated on issue #13 evidence.
- Issue #9 must implement only the M1 CAT allowlist, product-identify with `OM`, record
  firmware with `RVM`, use query-after-set verification, model documented response
  delays, and make every CAT uncertainty release/lock out an active lease.
- Issue #13 owns real-radio measurements and acceptance evidence. It is
  `human-required`; absence of physical results cannot be converted into agent
  evidence.

This specification selects no MCU, codec, CAT transceiver, isolation topology,
connector part, or final harness.
