# M1 control-firmware prototype and runbook

This document is the maintained implementation and operations record for the
radio-disconnected issue #12 firmware prototype. It applies ADRs 0003–0007, the
[M1 platform specification](m1-development-platform.md), the
[v0 protocol](../protocol/README.md), and the
[typed Elecraft CAT core](../crates/elecraft-cat/README.md) without changing them.

## Evidence boundary

The repository provides automated simulator, virtual-clock, document-derived CAT,
static descriptor, and source-configuration evidence. It does not claim:

- a successful nRF Connect SDK build or flash on a particular local installation;
- USB or BLE enumeration by a current iPhone, Android device, or desktop;
- simultaneous USB, BLE, I²S, QSPI, CAT, and safety timing on silicon;
- electrical, physical `PTT OUT`, watchdog-release, power, or current measurements;
- live KX2/KX3 behavior, RF output, or human-captured transcripts; or
- a safe radio connection.

Issues #13 and #18 remain the hard `human-required` boundaries for those results.

## Revisions and fixed fixtures

| Item | Checked-in revision |
| --- | --- |
| Development target | Nordic nRF5340 DK, PCA10095 application core; received board and SoC revisions must be recorded by the human operator |
| SDK/toolchain | nRF Connect SDK v3.3.0; record the SDK commit, Zephyr revision, compiler, and west manifest from the activated installation |
| BLE controller | Supported network-core HCI IPC image built through sysbuild |
| Protocol vectors | `protocol/vectors/v0.json`, `vector_version: 1`, protocol `0.0` |
| CAT fixtures | `crates/elecraft-cat/fixtures/document-derived/kx2.cat` and `kx3.cat`; neither is a radio capture |
| USB descriptor | Canonical 192-byte UAC1 configuration; endpoint `0x08` OUT and `0x88` IN, 96 bytes/ms, mono signed-16 48 kHz, `bDelay = 8` |
| I²S conversion | 12.288 MHz/48 kHz target, 24 valid bits in 32-bit words; mono16 widens and duplicates to stereo24, capture uses left-channel rounded/saturated stereo24→mono16 |
| PTT board default | Disabled; no pin or electrical circuit is inferred before #13 |
| USB MIDI | Not implemented; it remains conditional on #18 evidence and an updated ADR |

## Automated simulator and repository validation

Run all commands through the repository tasks:

```sh
mise run firmware:sim
mise run check
mise run ci
```

`firmware:sim` loads the repository vector file directly and exercises:

- both runtime ATT-value-size fragmentation cases and every malformed-envelope case;
- strict UTF-8 object parsing, duplicate-key, integer-only, and maximum-size rules;
- session establishment/replacement, boot identity, ordered operations, exact and
  altered duplicates, stale/replay/out-of-order/wrong-session faults, and the
  non-evicting 512-operation cache;
- intent, acquire, renew, release, expiry, 60-second cap, one-second continuous rearm,
  explicit fault recovery, and first-cause preservation on a virtual monotonic clock;
- BLE/session, host route, device audio, inhibit, profile, CAT, output-sense, reset,
  update, and watchdog fault actions as independent inputs; and
- typed CAT document fixtures plus pre-I/O rejection of raw and keying-capable
  requests.

`check_firmware.py`, reached through `mise run check`, independently confirms the five
fixed BLE UUIDs, completed-message dispatch and response identity, safety-snapshot
status rendering, byte parity between the C and Rust copies of the canonical UAC1
descriptor, the disabled PTT board default, and the unimplemented conditional-MIDI
boundary.

## nRF Connect SDK build

Activate an unmodified nRF Connect SDK v3.3.0 environment that supplies `west`, then
run:

```sh
mise run firmware:build
```

The task builds `firmware/nrf5340` for `nrf5340dk/nrf5340/cpuapp` with sysbuild and
places output under `target/nrf5340`. The application configuration:

- starts the application-core safety boundary before BLE;
- builds the network-core Bluetooth controller over HCI IPC;
- uses RTT instead of CDC, so diagnostics do not alter the USB descriptor;
- configures a dedicated 1 MHz application timer extended to 64 bits;
- configures a nominal 250 ms hardware watchdog that continues in sleep and debug
  halt; only a complete safety-service check can feed it;
- keeps the normal safety cadence at 25 ms; and
- keeps physical PTT output and bench fault injection disabled.

Record the full `west build` output, `west manifest --freeze`, compiler version,
linker map, flash/RAM use, board revision, SoC revision, and generated configuration.
A build result is not enumeration or timing evidence. This repository does not
silently fall back to UAC2, CDC, USB MIDI, a different board, or a different SDK.

The new and legacy Zephyr USB stacks remain an explicit build gate: the checked-in
descriptor bytes and conversion service are canonical, while physical UAC1 class
registration must be proven in #18 with an assigned VID/PID. The repository does not
fabricate a VID/PID or use Zephyr's sample identity.

## Flashing a radio-disconnected fixture

Before flashing, verify all of the following:

1. no radio is connected;
2. the TX-inhibit path is physically open when a later #13 fixture exists;
3. `CONFIG_RIGTETHER_PTT_OUTPUT_ENABLED=n` remains in the generated configuration;
4. the exact board serial/revision and built commit are recorded; and
5. `mise run firmware:build` completed from the pinned SDK.

Then run:

```sh
mise run firmware:flash
```

The task flashes the already-built `target/nrf5340` output and does not rebuild or
change configuration. A flash result alone is not completion evidence.

## BLE discovery and tracing

Use an independent BLE central or sniffer. Record tool, adapter, OS, connection,
negotiated ATT MTU, and capture revision. Verify:

| Role | UUID | Required access |
| --- | --- | --- |
| Service | `3dbbf179-78cb-4753-9c4e-092e2a4e1116` | primary service |
| hello | `686fd375-482c-407b-856d-3a5e757f9f03` | read |
| command | `f98fec49-c274-40d9-9b4b-38f5cef50e15` | write with response |
| response | `7ba61ae1-e6b0-4e3f-a718-fd31315cd876` | indicate |
| status | `f9ecbae1-3204-4870-b338-8f220e000585` | read, notify |

For each connection, compare `hello.command_frame_limit` with the actual negotiated
ATT MTU and confirm the characteristic-value limit is derived at runtime. Do not reuse
the value after reconnect. Capture fragment offsets, START/END flags, transfer IDs,
write responses, response indications, indication acknowledgements, and status
notifications separately. An ATT write response or indication acknowledgement is
delivery information only and must never be reported as extending a lease.

Completed command transfers enter the logical handler before the response is framed.
The response indication state machine retains each fragment until acknowledgement,
but release and lockout transitions occur synchronously before response delivery and
never wait for that acknowledgement.

Disconnect, second-session replacement, malformed framing, and stopped BLE host
processing must first request receive-safe release. Do not wait for a response,
indication acknowledgement, CAT completion, or the host scheduler before observing
the logical release event.

## USB and audio diagnostics

Until an assigned VID/PID is recorded for #18, use only static descriptor validation
and radio-disconnected desktop instrumentation. A physical capture must export raw
device, configuration, and string descriptor bytes and a decoded report. Verify:

- no BOS, device qualifier, IAD, CDC, MIDI, platform, vendor-specific, or unlisted
  string descriptor appears;
- the configuration is exactly 192 bytes and keeps endpoint `0x08`/`0x88`;
- both alternate settings carry one mono signed-16 48 kHz stream with 96-byte packets
  and `bDelay = 8`; and
- device and host-route health remain independent from BLE/session health.

RTT diagnostics keep USB configured/alternate state, SOF and packet counters, I²S
DMA/frame counters, clock adjustment, ring high/low water, underrun/overrun, sample
correction, and converter health as distinct fields. A sample insertion/drop,
underrun, overrun, missed DMA completion, stopped I²S clock, or two consecutive frames
outside the 2–14 ms safe band makes device audio unhealthy and requests release. It
is not hidden as clock recovery.

The designed `n → n+8` USB delay, actual clock error, ring stability, and simultaneous
transport behavior remain #18 measurements.

## Bounded logging

The live image uses deferred RTT logging and no USB CDC console. Safety events also
enter a fixed 128-record RAM ring containing a device-monotonic timestamp, event code,
and one bounded integer value. New records overwrite the oldest and increment an
independent dropped/overwritten count. No audio samples, raw host JSON, raw CAT, or
unbounded strings enter the safety log.

The on-device response cache uses a RAM index over CRC-checked, append-only records in
the fixed 4 MiB external-QSPI partition. It retains exact request and serialized
response bytes for all 512 negotiated operations and erases only after receive-safe
session replacement. QSPI build, erase/write latency, power-fail behavior, and
endurance remain measurement gates. A storage, integrity, or capacity fault ends the
session receive-safely; cache eviction within the negotiated session is never a
recovery strategy. The adjacent 2 MiB diagnostic partition is reserved but is not
claimed as persistent-log evidence.

## Fault injection

`CONFIG_RIGTETHER_FAULT_INJECTION` defaults off. A later labeled bench-only image may
inject stopped USB service, I²S DMA/clock loss, BLE host-processing loss, CAT timeout,
cache write failure, or normal scheduling delay. Every injection must be observable,
bounded, and release-only.

Fault injection must never:

- assert PTT or manufacture an active sense input;
- bypass lease, cap, inhibit, health, identity, ordering, or recovery checks;
- call the PTT writer from outside the safety service;
- feed the watchdog directly;
- introduce raw CAT or a second keying path; or
- be enabled in an unlabeled or release build.

## Safe recovery

Recovery is diagnostic and receive-only:

1. the safety service schedules commanded PTT inactive and TX audio muted immediately;
2. invalidate the lease and preserve the immutable first cause;
3. clear the external cause while keeping the radio disconnected and inhibit open;
4. observe sensed `PTT OUT` inactive; never fabricate it from commanded state;
5. replace a faulted protocol session and re-report host-route health;
6. after a continuous-cap fault, deliver release for the capped intent and observe one
   uninterrupted receive-safe second;
7. send `safety_recover` with the current `fault_id`; and
8. require a later fresh intent before any acquire.

Reconnect, reboot, session replacement, response delivery, or a new operation ID does
not restore intent, lease, or authority. If sensed `PTT OUT` remains active, firmware
cannot claim recovery; open the independent inhibit or disconnect the radio and leave
the fault latched for #13 investigation.

## Completion evidence format

A software-only issue #12 evidence record should contain:

- commit/PR and exact simulator/CAT/vector revisions;
- `mise run ci` result;
- shared-vector counts and virtual-clock result;
- static UUID/UAC1/PTT-default check result;
- any NCS build result with full toolchain manifest, explicitly labeled unflashed when
  no board was used;
- bounded-log and fault-injection configuration; and
- a direct statement that current-iPhone, electrical, physical PTT,
  watchdog-release, live-radio, and human-captured evidence were not performed.
