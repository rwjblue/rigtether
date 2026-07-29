# M1 development platform and instrumentable probe

This document fixes the development-hardware and audio strategy for the M1 bench
proof. It is an implementation input to issues #12, #13, and #18, not a production
BOM, purchase authorization, or claim of physical interoperability.

[ADR 0007](decisions/0007-use-nrf5340-dk-and-external-i2s-converters-for-m1.md)
records the decision. ADRs 0003–0006, the
[accepted architecture](architecture.md), the
[v0 control protocol](../protocol/README.md), and the
[transmit-safety contract](hardware-safety.md) remain normative.

## Evidence language

The tables use four evidence classes:

- **Documented:** stated by a primary manufacturer, silicon-vendor, USB-IF, Apple,
  Android, or upstream software document cited in the source table.
- **Inference:** an engineering conclusion from documented capabilities. It must be
  tested before becoming an interoperability claim.
- **Unknown:** the available primary sources do not settle the question.
- **Measurement required:** a named human bench result is required by issue #13 or
  #18.

No row marked inference, unknown, or measurement required is a feasibility claim.

## Requirements matrix

| M1 requirement | Proposed provision | Evidence and proof boundary |
| --- | --- | --- |
| Full-duplex class-compliant USB Audio and BLE GATT peripheral operation at the same time | nRF5340 application core owns USB, audio conversion, safety, and service logic; its network core runs the BLE controller over the supported inter-core HCI transport | The SoC documents the required USB, I²S, inter-core, memory, and BLE resources. Upstream supplies separate USB-audio and BLE samples. Their simultaneous operation, scheduling margin, radio coexistence, and independent recovery are inferences requiring #18 evidence. |
| Exact v0 BLE service without Apple assumptions or fixed MTU | Implement the four v0 characteristics and 16-byte fragment envelope exactly; obtain the connection ATT MTU at runtime and use `ATT_MTU - 3` as the characteristic-value limit | Zephyr documents runtime GATT MTU access. Protocol conformance is software-testable in #12; current-device behavior remains #18 evidence. |
| Observable audio service and conversion boundary | External Pmod I2S2 ADC and DAC on separate I²S signals; USB, playback ring, capture ring, clock servo, converter/DMA, and analog loopback counters are individually reported | The codec-module and nRF I²S capabilities are documented. Signal integrity, achieved clock error, analog quality, and buffer thresholds require measurement. |
| Firmware-monotonic lease timing and independent watchdog | A dedicated application-core 1 MHz timer is extended to 64 bits for device time. A non-stoppable application-core watchdog runs from LFCLK and is fed only after the safety service completes all checks | Timers and watchdog behavior are documented. Monotonicity across wrap, reset behavior, service latency, and passive PTT release remain #12/#13 tests. The watchdog is independent of the lease timer and normal task scheduler, but not of all SoC/common power faults. |
| Firmware language and toolchain | Use the vendor-supported nRF Connect SDK/Zephyr C interfaces for hardware-facing M1 USB, BLE, I²S, and safety integration. Keep protocol, CAT, host tools, fixtures, and future hardware abstractions Rust-compatible; introduce Rust on-device only behind measured, bounded interfaces | Community Embassy Rust supports the nRF5340 DK and nRF5340 USB peripheral, but no source demonstrates this exact UAC1/BLE/I²S topology. Not making Rust the initial hardware integration language is an accepted M1 compromise, not a production-language decision. |
| Typed CAT, one normally-open hardware PTT path, physical TX inhibit, and downstream `PTT OUT` sense | Reserve separate UART EasyDMA, command, inhibit, and sense signals at the DK headers. The M1 radio fixture implements the single normally-open PTT element; firmware has no CAT or audio keying command | Resource availability is documented. Pin assignment, switch/protection circuit, voltage domains, thresholds, radio grounds, and sensed state are intentionally deferred to source-backed #13 measurements. |
| Test access, fault injection, logging, and separate power/ground investigation | On-board J-Link SWD/RTT, current-measurement headers, QSPI log storage, short external I²S wiring, loopback jack, and build-gated fault-injection GPIOs | Board facilities are documented. Logging bandwidth, current, ground coupling, and fault behavior must be measured. |
| Credible future Android path | UAC1, full-speed, mono PCM Type I, signed 16-bit, 48 kHz, one isochronous endpoint in each direction | The format is inside Android's documented USB Audio Class 1 host-mode subset. That preserves a path; it is not an Android compatibility claim. |

## Platform comparison

| Candidate | Documented strengths | Limitations and unknowns | Result |
| --- | --- | --- | --- |
| **Nordic nRF5340 DK (PCA10095) plus external Pmod I2S2** | Dual Cortex-M33 application/network cores; BLE controller on the network core; full-speed USB with one isochronous IN and one isochronous OUT endpoint; simultaneous bidirectional I²S with EasyDMA and audio clock; 1 MiB application flash, 512 KiB RAM, external 64 Mbit QSPI flash; application watchdogs; on-board J-Link, headers, and current-measurement points | Upstream's available UAC1 device implementation is legacy and synchronous-only; a new-stack UAC1 migration may be needed. Simultaneous USB Audio, BLE, I²S, safety, and storage has not been demonstrated. The Pmod uses a discontinued DAC IC. | **Selected for the replaceable M1 probe.** It gives the clearest separation between BLE, USB, conversion, analog loopback, and safety observation without choosing a production circuit. |
| Nordic nRF5340 Audio DK | Same SoC; integrated CS47L63, USB-C, current monitors, stereo analog input, and mono analog output; vendor board support for audio development | Integrated routing makes the ADC/DAC boundary less replaceable and less convenient for separate probing. Analog output is mono. It still has the same UAC1 software-stack risk and is not evidence of the required simultaneous topology. | **First board-level substitute** if external I²S wiring or Pmod availability blocks the selected probe. |
| ST STM32WB5MM-DK plus external codec | Dual Cortex-M4/M0+ radio architecture; BLE 5.2, USB full-speed device, SAI, DMA, independent and window watchdogs, 1 MiB flash, 256 KiB RAM, debugger, and expansion headers | Lower RAM margin, a different dual-core integration model, and no primary-source demonstration found for the exact simultaneous UAC1/BLE/full-duplex audio use. An external codec and firmware integration are still required. | **MCU fallback**, triggered by an nRF-specific USB/BLE scheduling, clock, endpoint, or supported-stack blocker rather than by preference. |

The nRF5340 Audio DK and STM32WB5MM-DK comparisons establish credible alternatives;
neither is accepted as a production platform.

### Toolchain, availability, and cost

Availability and prices are volatile, locale-dependent snapshots accessed
2026-07-29. They exclude tax, tariff, shipping, cables, hub, instrumentation, and the
measurement-dependent radio fixture. They support owner review only and authorize no
order.

| Candidate | Rust viability and accepted toolchain | Availability snapshot | One-unit development-platform cost snapshot |
| --- | --- | --- | ---: |
| **nRF5340 DK plus Pmod I2S2** | Embassy publishes nRF5340 DK examples, an nRF5340 HAL, and an nRF5340 USB driver. It does not supply evidence for the exact UAC1/BLE/I²S/safety composition. Use nRF Connect SDK v3.3.0 C/Zephyr APIs for hardware-facing M1 work; Rust remains preferred for reusable protocol/CAT/tooling where a narrow interface avoids a second PTT owner or timing ambiguity. | Mouser listed 498 nRF5340 DKs ready to ship; Digilent listed the Pmod as purchasable and DigiKey listed 371 immediately available. The module remains lifecycle-risky because its CS4344 is discontinued. | **$74.53:** $47.53 DK plus $27.00 manufacturer-listed Pmod |
| nRF5340 Audio DK | Same community Rust SoC support and same missing exact-topology evidence. Its vendor path is also nRF Connect SDK, so it does not reduce the M1 Rust integration risk. | DigiKey listed 88 immediately available; Nordic identifies it as a current development product. | $172.57 |
| STM32WB5MM-DK plus Pmod I2S2 | Embassy advertises STM32WB wireless support, but no source demonstrates the exact UAC1/BLE/audio/safety composition on this board. The accepted fallback would begin with STM32CubeWB C integration and keep the same Rust-neutral service boundaries. | DigiKey listed 227 immediately available and ST marked the board active; the same Pmod availability/lifecycle caveat applies. | $114.00: $87.00 DK plus $27.00 Pmod |

The selected combination is both the lowest listed base-platform cost and the most
readily stocked of the compared MCU boards at this snapshot. Cost does not override
the UAC1, clock, safety, or physical-evidence gates. If its stock disappears, the
listed board/module substitutes trigger a fresh quote and capability check rather
than an automatic purchase.

## Selected development hardware

Use one **Nordic nRF5340 DK, PCA10095**, with the nRF5340 application core as the
USB-device, audio-service, protocol-service, CAT-adapter, diagnostics, and sole
PTT-safety compute owner. Run the supported BLE controller on the network core and
connect it to the application-core host through the upstream HCI IPC path.

Pin firmware to **nRF Connect SDK v3.3.0** for the first #12/#18 build record. Record
the SDK commit, Zephyr revision, compiler version, board revision, SoC revision, and
configuration in every result. The pin is reproducibility input, not a promise to
retain that SDK: the legacy Zephyr UAC1 implementation is deprecated, so the first
firmware spike must prove that the exact descriptor below builds and enumerates.

Use one **Digilent Pmod I2S2, SKU 410-379**, as the development-only external
conversion module. It exposes a Cirrus Logic CS5343 stereo ADC and CS4344 stereo DAC
through separate I²S paths and separate 3.5 mm line input/output jacks. Configure the
nRF as I²S clock master:

- 12.288 MHz master clock from `HFCLKAUDIO`;
- 48 kHz word-select rate;
- I²S format, 24 valid bits in 32-bit DMA words; and
- simultaneous ADC receive and DAC transmit.

The USB-to-I²S conversion boundary is explicit:

- playback widens each signed 16-bit USB mono sample to signed 24-bit and duplicates it
  to both DAC channels;
- capture uses the ADC left channel, rounds and saturates signed 24-bit to signed
  16-bit, and keeps right-channel activity only as diagnostics;
- no conversion state, unused channel, CAT operation, or audio amplitude can request
  transmit; and
- analog levels, clipping, latency, noise, and ground behavior remain #13
  measurements.

The Pmod choice is intentionally replaceable. The CS4344 is listed as discontinued by
Cirrus Logic; do not freeze it into Rev A.

## Exact M1 USB Audio proposal

Issue #18 must validate the canonical configuration descriptor below byte-for-byte.
The device descriptor is canonical except for the assigned VID/PID and derived
per-board serial. The firmware fixture must export raw device, configuration, and
string descriptor bytes plus a decoded report as test artifacts.

### Device and configuration

| Field | M1 value |
| --- | --- |
| USB revision | USB 2.0 full-speed, `bcdUSB = 0x0200` |
| Device class | Per-interface: class, subclass, and protocol all `0x00` |
| Endpoint 0 | 64-byte maximum packet |
| Device release | Initial probe, `bcdDevice = 0x0001`; increment for descriptor-incompatible probe revisions |
| Audio class revision | USB Audio Device Class 1.0, `bcdADC = 0x0100` |
| Configuration | Value 1; three interfaces; `wTotalLength = 192`; bus powered; no remote wake |
| Declared current | `bMaxPower = 250` USB 2.0 units, or 500 mA |
| Strings | Index 0 language `0x0409`; index 1 `RigTether contributors`; index 2 `RigTether M1 audio probe`; index 3 stable per-board serial; every other string index zero |

The 500 mA descriptor is a ceiling for the configured development device, not a
measured load and not a promise that a phone will supply it. Enumeration, actual
current, inrush, suspend current, disconnect behavior, and phone acceptance are #18
evidence.

USB vendor and product IDs are a pre-#18 blocker, not a field to fabricate. Zephyr
documents that its `0x2FE3` VID is limited to Zephyr project samples and must not be
used by a vendor-integrated application. Before flashing a current-device fixture,
obtain an assigned open-source project PID or an owner-provided VID/PID, record both
in the #18 issue, and freeze them in the raw descriptor artifact. All class,
configuration, interface, terminal, format, and endpoint bytes are fixed below; an
assigned identity does not authorize changing them.

The 18-byte device descriptor is:

| Offset | Field | Exact value |
| ---: | --- | --- |
| 0 | `bLength`, `bDescriptorType` | `12 01` |
| 2 | `bcdUSB` | `00 02` |
| 4 | class, subclass, protocol | `00 00 00` |
| 7 | `bMaxPacketSize0` | `40` |
| 8 | `idVendor`, little-endian | Assigned two-byte value |
| 10 | `idProduct`, little-endian | Assigned two-byte value |
| 12 | `bcdDevice` | `01 00` |
| 14 | manufacturer, product, serial indexes | `01 02 03` |
| 17 | `bNumConfigurations` | `01` |

String descriptor 0 is exactly `04 03 09 04`. String descriptors 1 and 2 are exactly:

```text
2E 03 52 00 69 00 67 00 54 00 65 00 74 00 68 00 65 00 72 00 20 00
63 00 6F 00 6E 00 74 00 72 00 69 00 62 00 75 00 74 00 6F 00 72 00
73 00

32 03 52 00 69 00 67 00 54 00 65 00 74 00 68 00 65 00 72 00 20 00
4D 00 31 00 20 00 61 00 75 00 64 00 69 00 6F 00 20 00 70 00 72 00
6F 00 62 00 65 00
```

Encode index 3 as exactly 16 uppercase hexadecimal characters: `DEVICEID[0]` followed
by `DEVICEID[1]` in register order, each rendered as eight
most-significant-digit-first characters and then UTF-16LE. Its descriptor starts
`22 03`. Return no other string, device-qualifier, BOS, interface-association,
platform, or vendor-specific descriptor.

### AudioControl graph

Interface 0 is AudioControl subclass `0x01`, protocol `0x00`, with `iInterface = 0`.

- The class-specific header has `bInCollection = 2`, `baInterfaceNr = {1, 2}`, and
  `wTotalLength = 70`.
- Playback: entity 1 USB Streaming Input Terminal `0x0101` → entity 2 mute-only
  Feature Unit → entity 3 Line Connector Output Terminal `0x0603`.
- Capture: entity 4 Line Connector Input Terminal `0x0603` → entity 5 mute-only
  Feature Unit → entity 6 USB Streaming Output Terminal `0x0101`.
- Both paths declare one logical channel with `wChannelConfig = 0`.
- Each Feature Unit has `bControlSize = 1`, master-channel mute `0x01`, and no
  per-channel control.
- Every `bAssocTerminal`, `iChannelNames`, `iTerminal`, and `iFeature` is zero.
- Do not advertise volume controls, automatic gain, clock selection, or sample-rate
  controls that the probe does not implement.

### AudioStreaming interfaces

| Direction | Interface | Alternate 0 | Alternate 1 | Endpoint |
| --- | --- | --- | --- | --- |
| Host playback to RigTether | 1 | Zero endpoints | Terminal link 1; PCM Type I `wFormatTag = 0x0001`; mono; signed 16-bit; 2-byte subframe; one discrete rate encoded as `80 BB 00` (48,000 Hz) | Dedicated isochronous synchronous data OUT `0x08`; `bmAttributes = 0x0D`; `wMaxPacketSize = 0x0060` (96 bytes); `bInterval = 1`; `bRefresh = 0`; `bSynchAddress = 0`; class endpoint attributes/lock delay zero |
| RigTether capture to host | 2 | Zero endpoints | Terminal link 6; PCM Type I `wFormatTag = 0x0001`; mono; signed 16-bit; 2-byte subframe; one discrete rate encoded as `80 BB 00` (48,000 Hz) | Dedicated isochronous synchronous data IN `0x88`; `bmAttributes = 0x0D`; `wMaxPacketSize = 0x0060` (96 bytes); `bInterval = 1`; `bRefresh = 0`; `bSynchAddress = 0`; class endpoint attributes/lock delay zero |

Both interfaces use class `0x01`, subclass `0x02`, protocol `0x00`, and
`iInterface = 0`. Both advertise `bDelay = 8`, matching the designed eight USB
full-speed frame internal ring delay in each direction. The canonical 192-byte
configuration descriptor, including every zero-valued optional field, is:

```text
09 02 C0 00 03 01 00 80 FA
09 04 00 00 00 01 01 00 00
0A 24 01 00 01 46 00 02 01 02
0C 24 02 01 01 01 00 01 00 00 00 00
09 24 06 02 01 01 01 00 00
09 24 03 03 03 06 00 02 00
0C 24 02 04 03 06 00 01 00 00 00 00
09 24 06 05 04 01 01 00 00
09 24 03 06 01 01 00 05 00
09 04 01 00 00 01 02 00 00
09 04 01 01 01 01 02 00 00
07 24 01 01 08 01 00
0B 24 02 01 01 02 10 01 80 BB 00
09 05 08 0D 60 00 01 00 00
07 25 01 00 00 00 00
09 04 02 00 00 01 02 00 00
09 04 02 01 01 01 02 00 00
07 24 01 06 08 01 00
0B 24 02 01 01 02 10 01 80 BB 00
09 05 88 0D 60 00 01 00 00
07 25 01 00 00 00 00
```

This is inside Android's documented UAC1 PCM host-mode subset and uses the nRF5340's
dedicated endpoint number 8 for its documented single isochronous endpoint in each
direction. Apple documents that a synchronous USB audio function must discipline its
sample clock to USB SOF or provide sample-rate conversion. Therefore the proposal
includes a measured digital clock servo; merely selecting `HFCLKAUDIO` is insufficient
evidence.

Timestamp USB SOF and I²S frame events with a dedicated timer. Adjust `HFCLKAUDIO`
within its documented fine-adjustment range to hold each 16 ms playback/capture ring
at an 8 ms target. A diagnostic sample insertion/drop is allowed only to expose a
failed experiment: it makes `clock_healthy` and `buffers_healthy` false and initiates
the ADR 0004 receive-safe path. It is not normal clock recovery and cannot be hidden
as successful audio.

The initial thresholds for #18 are:

- target fill and advertised UAC1 `bDelay`: 8 USB frames (8 ms);
- warning band: below 4 ms or above 12 ms;
- unsafe band: below 2 ms or above 14 ms for two consecutive USB frames;
- immediate unhealthy state: underrun, overrun, missed DMA completion, stopped I²S
  frame clock, or any diagnostic sample insertion/drop.

Issue #18 must measure whether these thresholds are stable under simultaneous BLE
traffic and fault tests. It must also verify the UAC1 Section 3.4 mapping: the first
playback sample received in USB frame `n` is first reproduced in frame `n + 8`, and
the first capture sample fully acquired after `SOF n` is first transmitted in frame
`n + 8`, within the specification's phase-jitter bound. The external converter's
pipeline delay and DMA scheduling are not yet measured. If either direction cannot
meet the designed integer-frame delay, the fixture fails and ADR 0007/descriptors
must be revised before a support claim. A result may tighten health thresholds; it
must not silently relax the ADR 0004 release bounds.

## BLE, memory, timing, and debug constraints

### BLE and protocol

- Implement the exact service and characteristic UUIDs, permissions, indications,
  notifications, envelope, logical JSON, session rules, typed CAT allowlist, and PTT
  semantics in the [v0 contract](../protocol/README.md).
- Freeze the v0 service UUID
  `3dbbf179-78cb-4753-9c4e-092e2a4e1116` and hello
  (`686fd375-482c-407b-856d-3a5e757f9f03`), command
  (`f98fec49-c274-40d9-9b4b-38f5cef50e15`), response
  (`7ba61ae1-e6b0-4e3f-a718-fd31315cd876`), and status
  (`f9ecbae1-3204-4870-b338-8f220e000585`) characteristic UUIDs. Their exact
  properties remain normative in the protocol document.
- Read the negotiated ATT MTU for each connection. Never compile a fixed Apple MTU or
  reuse a cached peer MTU after reconnect.
- Keep BLE link health independent of USB configured/streaming health. USB reset must
  not invent BLE health; BLE disconnect must not conceal USB state.
- Treat an application-core crash as a common-mode fault even though the BLE controller
  runs on the network core. The physical PTT path must release independently.

### DMA, clock, and scheduling

- Give USB isochronous service and the safety service bounded, higher-priority
  scheduling than JSON, storage, logs, CAT polling, or diagnostics.
- Use EasyDMA for both I²S directions and a separate EasyDMA UART for CAT.
- Use fixed-size pools after boot for USB packets, audio rings, GATT fragments,
  serialized responses, and log records. No allocation failure may preserve PTT.
- Run the M1 probe without deep sleep. USB suspend, USB reset, stopped SOF, I²S clock
  loss, and BLE controller failure are distinct injected faults.
- Reserve the application core's audio clock for I²S. Record measured clock-servo
  range, phase error, ring occupancy, and every correction decision.

### Memory budget

Treat these as build ceilings to be verified from the link map:

| Resource | Initial ceiling |
| --- | --- |
| Application image, constants, and read-only data | 512 KiB internal flash |
| Runtime data, stacks, heaps/pools, and audio buffers | 320 KiB internal RAM |
| Network-core controller image/data | Upstream partition; record actual map |
| Exact serialized v0 response cache and identity material | 4 MiB external QSPI |
| Circular diagnostic/event log | 2 MiB external QSPI |
| QSPI reserve and metadata | Remaining capacity; no overcommit |

The cache uses a RAM index and append-only flash records. A flash, integrity, or
capacity fault ends the session receive-safely; eviction inside the negotiated v0
session limit is forbidden. Actual serialized-message distribution, erase latency,
power-fail behavior, and endurance remain #12 measurements before the cache design is
accepted.

### Lease time and watchdog

- Derive protocol `device_time_ms` and lease deadlines from a dedicated 1 MHz
  application-core timer extended to 64 bits. Test counter wrap. Do not derive leases
  from BLE events, USB frames, wall time, or the network core.
- Configure application watchdog 0 for a nominal 250 ms timeout, running in sleep and
  debug halt and locked after start. Only the dedicated safety service may feed it,
  after validating lease, inhibit, sensed output, audio health, protocol state, and
  its own scheduling deadline.
- Keep the normal safety-service cadence at or below 50 ms. A watchdog reset must drive
  the sole PTT command inactive before or as reset occurs, and the passive normally
  open hardware path must remain released without firmware.
- Preserve the normative 500 ms maximum accepted lease, 250 ms host renewal target,
  100 ms release target after expiry/safety event, 500 ms independent-watchdog release
  bound, 60 s continuous-authority cap, and safe rearm sequence from ADR 0004.

The 250 ms watchdog setting is a proposal for measurement, not proof of passive
release. Issue #13 records reset-to-release timing with the final development fixture.

### Observability and fault injection

Expose these independently through v0 health/status and timestamped logs:

- USB configured state, active alternate settings, reset/suspend/SOF counters, and
  isochronous packet counts;
- I²S frame/DMA counters, PLL adjustment, measured clock error, ring high/low water,
  underruns, overruns, and diagnostic sample corrections;
- `configured`, `tx_stream`, `clock`, `buffers`, and `converter` audio-health causes,
  including the first transition to unhealthy;
- BLE connection, negotiated MTU, indication backlog, retransmission/session errors,
  and controller reset;
- lease deadline, service-check duration, watchdog feeds/resets, inhibit state,
  commanded PTT, downstream sensed `PTT OUT`, and first-cause lockout;
- CAT request type, profile, result, latency, and rejected unsafe input without
  recording sensitive host data; and
- reset reason, boot ID, build identity, and dropped-log count.

Use on-board J-Link SWD and RTT for primary live debug so that a CDC interface does not
alter the USB descriptor under test. Mirror bounded records to QSPI. Reserve test-point
GPIOs for USB-SOF timing, I²S frame timing, safety-service execution, commanded PTT,
physical inhibit, sensed `PTT OUT`, and build-gated fault triggers.

Fault injection must be compiled only into a labeled bench image and be physically
unavailable to a release build. It may stop USB service, I²S DMA/clock, BLE host
processing, CAT responses, QSPI writes, or normal scheduling. It may never bypass
lease validation, assert PTT, feed the watchdog directly, or become another PTT owner.

`converter_healthy` means only that required I²S clocks and DMA are active and the
most recent commanded digital self-test passed. The Pmod has no accepted analog
health readback; analog loopback and radio-side behavior remain human evidence.

## Bench power and ground options

### Externally powered baseline

Use a powered, USB 2.0-capable USB-C host hub between the phone and the nRF USB-device
connector. Power the DK from the hub's downstream VBUS and keep the USB descriptor
bus-powered. Measure VBUS voltage/current and confirm there is no second supply or
back-power path before connecting the phone.

For initial desktop-only instrumentation, Nordic documents the DK's dedicated nRF USB
input and the board current-measurement headers. If an external 1.7–3.6 V supply is
used at the documented DK input while USB supplies signaling/VBUS detection, follow
the board guide exactly and verify the board revision's power-switch behavior first.

The DK may power the Pmod from a documented board rail only after measuring the
module's demand and remaining below the DK guide's applicable external-load limit.
Never power the Pmod from the DK and another source simultaneously.

### Phone-powered experiment

Apple documents up to 4.5 W accessory output for supported USB-C iPhone models, but
that does not establish this board/module load, inrush, enumeration, cable loss, or
current-device behavior. Attempt direct phone power only in #18 after:

1. recording the exact phone, OS, cable, DK, Pmod, and firmware revisions;
2. measuring configured, streaming, BLE-active, transmit-fixture-active, suspend, and
   peak/inrush current on an external host or analyzer;
3. confirming the development descriptor and measured load stay within documented
   USB and Apple limits; and
4. starting with the radio disconnected and the physical TX inhibit open.

Phone refusal, route loss, resets, excessive current, or unstable voltage triggers the
powered-hub baseline. It is not a firmware reason to change PTT safety.

### Ground investigation

Keep USB ground, DK ground, codec analog ground, CAT reference, microphone/PTT
reference, radio audio return, and chassis/shield as separate named nets in the bench
record until #13 measurements justify any connection. Record every temporary bond,
instrument earth, cable shield, and supply return. Do not infer a safe connection from
connector shape or another radio model.

## M1 development BOM and substitutes

This list identifies a reproducible probe. It authorizes no order.

| Qty | M1 item | Purpose | Substitute or boundary |
| ---: | --- | --- | --- |
| 1 | Nordic nRF5340 DK, PCA10095 | MCU, BLE, USB device, I²S, CAT, safety, debug, QSPI | nRF5340 Audio DK for an integrated-codec probe; STM32WB5MM-DK plus external codec after an explicit fallback trigger |
| 1 | Digilent Pmod I2S2, SKU 410-379 | Replaceable stereo line ADC/DAC boundary | PJRC Teensy Audio Adapter Rev D2/Rev D2.1 with SGTL5000 after clock, voltage, wiring, and driver review; or nRF5340 Audio DK CS47L63 |
| 1 | Data-capable USB-C-host to the DK's dedicated nRF USB-device connector cable | Phone/hub USB Audio link | Exact connector must match the received PCA10095 revision and be recorded |
| 1 | Powered USB 2.0-capable USB-C host hub and documented supply | Externally powered phone baseline | Any equivalent hub only after data-role and power-path verification |
| 1 | USB power analyzer with transparent data path | Current, voltage, inrush, and reset observation | Nordic Power Profiler Kit II for suitable board-rail measurements; neither replaces USB protocol capture |
| 1 | Short 2×6 Pmod cable/breakout or equivalent short 0.1-inch jumpers | Separate I²S signal access | Logic-analyzer breakout with the same voltage and short-wire constraints |
| 1 | 3.5 mm TRS line-level patch cable | ADC-to-DAC analog loopback | Calibrated generator/load during #13 |
| 1 set | Breadboard/terminal breakout, labeled jumpers, removable two-pin TX-inhibit shunt, and test leads | Radio-disconnected PTT/sense fixture and named-ground investigation | A purpose-built fixture only after #13 settles the electrical circuit |
| 1 set | Oscilloscope or logic-analyzer probes and current-limited bench instrumentation | Clocks, DMA markers, reset, PTT release, and ground measurements | Existing calibrated instruments; record model and setup |

The BOM deliberately does **not** select the radio-side analog conditioning, PTT
switch, `PTT OUT` sensing protection, CAT level interface, ESD parts, current limits,
or ground network. Those values and components depend on official specifications and
the human measurements owned by #13. No radio connection is made by #18.

## Smallest #18 hardware/firmware probe

The smallest current-device probe is the selected DK, Pmod, short I²S wiring, one
analog loopback cable, powered hub/power instrumentation, physical inhibit held open,
and no radio.

Use staged, reproducible firmware images:

1. **Descriptor fixture:** exact UAC1 descriptors, deterministic USB playback/capture
   digital loopback, raw descriptor export, endpoint/SOF counters, and no BLE/PTT.
2. **Clocked audio fixture:** add external I²S, conversion, ring buffers, clock servo,
   analog loopback, DMA markers, and audio health; physical inhibit remains open.
3. **Simultaneous-transport fixture:** add the exact v0 GATT service, runtime MTU,
   network-core BLE controller, concurrent traffic, independent health, logs, and
   fault injection; PTT output remains compile-time disabled.
4. **Safety fixture:** add the radio-disconnected normally-open switch/sense load,
   monotonic lease timer, inhibit, independent watchdog, and all receive-safe fault
   tests. Radio-side electrical insertion remains #13.

Each stage keeps the prior stage's artifacts and records CPU load, stack high-water
marks, memory/link map, current, clock/ring telemetry, resets, and failures. A failure
is evidence; do not tune it out without preserving the original result.

### Conditional USB MIDI fallback capability

USB Audio remains the primary media transport and BLE remains the primary control
transport. The nRF USB peripheral documents spare bulk/interrupt endpoints after the
two isochronous audio endpoints, so the hardware can support a conditional MIDI
experiment. Software composition is not yet proven.

Prepare a separate, build-gated #18 descriptor target that attempts to add one USB
MIDI 1.0 streaming interface with one bulk OUT endpoint `0x02` and one bulk IN endpoint
`0x82` to the unchanged audio configuration. Initially it carries only a deterministic
MIDI loopback/identity probe. If and only if current-device evidence triggers the ADR
0003 fallback, a later ADR may define MIDI framing for the same v0 logical JSON; it
must not reuse the BLE fragment envelope or become a raw CAT/PTT channel.

nRF Connect SDK v3.3.0's upstream UAC1 fixture uses the deprecated legacy USB device
stack, while its current MIDI sample uses the newer device stack. The first build
spike must therefore answer whether a supported, reviewable composite UAC1+MIDI 1.0
image can be produced without maintaining an unsafe USB fork. If not, #18 records the
software blocker and may use a standalone MIDI identity/loopback image only to learn
current-device enumeration behavior. It may not present that as a composite fallback
success.

## Risks, blockers, and fallback triggers

| Risk or blocker | Trigger | Required response |
| --- | --- | --- |
| Deprecated legacy Zephyr UAC1 stack | Exact UAC1 fixture cannot build, enumerate, pass descriptor checks, or has no acceptable maintenance path | Spike a minimal UAC1 implementation on the new stack. If still blocked, evaluate the STM32WB SDK/platform or a narrowly maintained class implementation; do not silently select UAC2. |
| USB/audio clock-domain failure | Clock servo cannot sustain 48 kHz full duplex without correction, ring fault, or safety release under the #18 stress matrix | Preserve traces; test documented clock-control limits and scheduling. Then trigger MCU/codec/topology comparison. Do not weaken health thresholds or claim audio success. |
| Simultaneous USB, BLE, I²S, storage, and safety overload | Missed USB frames, BLE loss correlated with audio, safety deadline miss, watchdog reset, or insufficient memory margin | Remove nonessential logs/storage from the real-time path, measure again, then trigger the STM32 or split-controller alternative if isolation remains inadequate. |
| Pmod CS4344 obsolescence or signal-integrity failure | Module unavailable, clocks/wiring cannot meet documented timing, or repeatable conversion faults persist | Use the SGTL5000 or nRF5340 Audio DK substitute and rerun the complete audio/clock evidence; never promote the Pmod circuit to Rev A. |
| Phone power instability | Enumeration refusal, route loss, brownout/reset, or current outside documented limits | Use the powered-hub baseline and retain power as an open Rev A requirement. |
| USB MIDI composite unsupported | Reviewable UAC1+MIDI image cannot be built or current iPhone evidence rejects it | Record the result. MIDI remains conditional and cannot replace BLE without an ADR and exact protocol framing. |
| Radio electrical unknowns | Any required KX2/KX3 voltage, impedance, bias, ground, or threshold lacks an official value or measurement | Keep the radio disconnected and #13 `human-required`; do not select a production component or infer from connector appearance. |

## Unresolved measurements

Issue #18 owns current-device, radio-disconnected evidence for:

- exact USB descriptors and full-duplex audio enumeration/routing;
- simultaneous USB Audio and exact v0 BLE operation with runtime MTU;
- clock-servo range, SOF/I²S error, ring stability, latency, and fault thresholds;
- independent USB/BLE failures, app lifecycle, reconnect, interference, and recovery;
- direct-phone and powered-hub voltage/current/inrush/suspend behavior;
- staged MIDI identity/composite behavior if the build gate succeeds; and
- exact phone, iOS, cable, hub, board, silicon, SDK, and firmware revisions.

Issue #13 owns human bench evidence for:

- Pmod and substitute analog input/output range, clipping, noise, latency, and ground
  current;
- radio audio, CAT, PTT, sense, bias, reference, chassis, and protection values;
- normally-open switch, physical TX inhibit, downstream sensed `PTT OUT`, watchdog,
  reset, power-loss, and every ADR 0004 release time;
- current-limiting, ESD, test-point, and practical TX-inhibit circuit behavior; and
- human-supervised KX2/KX3 operation at minimum practical power into a suitable dummy
  load.

No fabrication output becomes buildable from this development-platform selection.

## Primary sources

All sources were accessed 2026-07-29.

| Source | Facts used |
| --- | --- |
| [Nordic nRF5340 product specification — key features](https://docs.nordicsemi.com/r/bundle/ps_nrf5340/page/keyfeatures_html5.html) | Core, memory, BLE, USB, serial, I²S, timer, and GPIO capabilities |
| [Nordic nRF5340 USBD](https://docs.nordicsemi.com/r/bundle/ps_nrf5340/page/chapters/usbd/doc/usbd.html) | Full-speed operation, dedicated endpoint number 8 isochronous IN/OUT pair, other endpoints, double buffering, and PHY behavior |
| [Nordic nRF5340 I²S](https://docs.nordicsemi.com/r/bundle/ps_nrf5340/page/i2s.html) | Bidirectional I²S, EasyDMA, sample widths, master clock, and audio-clock support |
| [Nordic nRF5340 clock](https://docs.nordicsemi.com/r/bundle/ps_nrf5340/page/chapters/clock/doc/clock.html) | `HFCLKAUDIO` frequency bands and fine adjustment |
| [Nordic nRF5340 watchdog](https://docs.nordicsemi.com/r/bundle/ps_nrf5340/page/wdt.html) and [reset](https://docs.nordicsemi.com/r/bundle/ps_nrf5340/page/chapters/reset/doc/reset.html) | Locked watchdog behavior, LFCLK timing, reset scope, and reset reasons |
| [Nordic nRF5340 DK power](https://docs.nordicsemi.com/r/bundle/ug_nrf5340_dk/page/UG/nrf5340_DK/hw_power_supply.html), [current measurement](https://docs.nordicsemi.com/r/bundle/ug_nrf5340_dk/page/ug/dk/hw_measure_current.html), and [external-board power](https://docs.nordicsemi.com/r/bundle/ug_nrf5340_dk/page/ug/dk/ext_programming_support_p20.html) | Documented DK inputs, measurement headers, external-supply cautions, and external-load boundary |
| [Zephyr nRF5340 DK board support](https://docs.zephyrproject.org/latest/boards/nordic/nrf5340dk/doc/index.html), [Bluetooth samples](https://docs.zephyrproject.org/latest/samples/bluetooth/bluetooth.html), and [GATT server API](https://docs.zephyrproject.org/latest/doxygen/html/group__bt__gatt__server.html) | Board/debug support, HCI IPC pattern, and runtime ATT MTU |
| [Zephyr USB VID/PID policy](https://docs.zephyrproject.org/latest/services/connectivity/usb/device_next/vid_pid.html) | Zephyr sample VID scope and allocation requirement |
| [nRF Connect SDK v3.3.0](https://github.com/nrfconnect/sdk-nrf/releases/tag/v3.3.0) | Reproducible SDK release and upstream Zephyr revision |
| [Embassy project](https://github.com/embassy-rs/embassy), [nRF5340 HAL](https://docs.embassy.dev/embassy-nrf/0.9.0/nrf5340-app-ns/index.html), and [nRF5340 USB module](https://docs.embassy.dev/embassy-nrf/0.9.0/nrf5340-app-ns/usb/index.html) | Community Rust board examples, HAL coverage, and USB-driver availability; no exact RigTether-topology claim |
| [Zephyr legacy USB device stack](https://docs.zephyrproject.org/latest/connectivity/usb/device/usb_device.html) | UAC1 `bcdADC = 0x0100`, synchronous-only limitation, and deprecation status |
| [Zephyr UAC2 external-I²S sample](https://docs.zephyrproject.org/latest/samples/subsys/usb/uac2_implicit_feedback/README.html) | Upstream nRF5340 DK plus external I²S ADC/DAC precedent; not evidence for the selected UAC1 topology |
| [Digilent Pmod I2S2](https://digilent.com/shop/pmod-i2s2-stereo-audio-input-and-output/) | SKU, CS5343/CS4344, line jacks, stereo, resolution, rates, separate I²S paths, current manufacturer listing, and $27.00 price snapshot |
| [Cirrus Logic CS4344](https://www.cirrus.com/products/cs4344-45-48/) | DAC lifecycle status and device documentation |
| [Nordic nRF5340 Audio DK](https://docs.nordicsemi.com/r/bundle/ug_nrf5340_audio/page/ug/nrf5340_audio/intro.html) and [Cirrus Logic CS47L63 data sheet](https://statics.cirrus.com/pubs/proDatasheet/CS47L63_DS1249F2.pdf) | Integrated-codec substitute facilities |
| [PJRC Teensy Audio Adapter](https://www.pjrc.com/store/teensy3_audio.html) | SGTL5000 substitute facilities, line I/O, and wiring constraints |
| [ST STM32WB5MM-DK](https://www.st.com/en/evaluation-tools/stm32wb5mm-dk.html) and [STM32WB55VG](https://www.st.com/en/microcontrollers-microprocessors/stm32wb55vg.html) | Alternative board/SoC BLE, USB, SAI, DMA, memory, watchdog, debug, and expansion capabilities |
| [Mouser nRF5340 DK listing](https://www.mouser.com/en/ProductDetail/Nordic-Semiconductor/NRF5340-DK) | Authorized-distributor one-unit price and stock snapshot |
| [DigiKey Pmod I2S2 listing](https://www.digikey.com/en/product-highlight/d/digilent/pmod-i2s2-stereo-audio-input-and-output), [nRF5340 Audio DK listing](https://www.digikey.com/en/products/detail/nordic-semiconductor-asa/NRF5340-AUDIO-DK/16399476), and [STM32WB5MM-DK listing](https://www.digikey.com/en/products/detail/stmicroelectronics/STM32WB5MM-DK/13693662) | Authorized-distributor availability and one-unit price snapshots |
| [USB-IF Audio Device Class 1.0](https://www.usb.org/sites/default/files/audio10.pdf), [Audio Data Formats 1.0](https://www.usb.org/sites/default/files/frmts10.pdf), and [Terminal Types 1.0](https://www.usb.org/sites/default/files/termt10.pdf) | UAC1 descriptor, PCM Type I, endpoint, channel, and terminal definitions |
| [USB 2.0 specification](https://www.usb.org/document-library/usb-20-specification) | Device/configuration descriptors, full-speed endpoint rules, and `bMaxPower` units |
| [USB-IF MIDI Devices 1.0](https://www.usb.org/sites/default/files/midi10.pdf) | Conditional MIDI 1.0 streaming descriptor baseline |
| [Apple TN3190: USB audio device design considerations](https://developer.apple.com/documentation/technotes/tn3190-usb-audio-device-design-considerations) | UAC design, full-speed timing, endpoint synchronization, and clock-discipline constraints |
| [Apple: Charge and connect with the USB-C connector](https://support.apple.com/en-us/105099) | Documented supported-iPhone accessory power ceiling |
| [Android USB digital audio](https://source.android.com/docs/core/audio/usb) | Documented Android UAC1 host-mode PCM subset and powered-hub guidance |
