# 0003 — Use USB Audio Class plus BLE GATT control

- Status: Accepted
- Date: 2026-07-29

## Context

Bidirectional audio benefits from a standard low-latency host audio path. Radio CAT and
PTT require little bandwidth but must be available to a distributable iOS application.
An ordinarily distributed App Store application must use public APIs and cannot assume
desktop serial drivers, private USB access, MFi participation, or approval for a
restricted entitlement.

The feasibility prototype also needs a narrow decision, not a permanent public
protocol or production USB topology. Current-device tests still own exact descriptors,
formats, latency, RF coexistence, power, and App Store submission evidence.

iPhone is the first implementation and validation target. Android is a planned future
host, so the feasibility choice must not require Apple-only accessory protocols or make
Apple framework behavior part of the hardware, firmware, or device protocol contract.

## Decision

Use two independent logical transports:

- A class-compliant USB Audio function carries continuous receive and transmit audio.
  The iOS application uses AVFAudio to observe the `usbAudio` input and output route.
  The exact USB Audio Class revision, topology, formats, and power design remain M1
  selections backed by device tests.
- A custom Bluetooth Low Energy GATT service carries discovery, protocol version,
  capabilities, CAT/control commands, status, errors, configuration needed by the
  proof, and bounded PTT leases. The iPhone is the BLE central/GATT client; RigTether is
  the peripheral/GATT server.

The USB-C cable is the only physical phone tether. BLE does not add another cable.
Audio and control connection state remain independent, and audio can never assert PTT.
The interface, not the app scheduler or BLE connection, enforces lease expiry and
receive-safe behavior.

The minimum transport-facing contract is:

1. Advertise one identifiable RigTether GATT service and expose protocol version,
   capabilities, and session identity.
2. Provide an acknowledged command/response path and asynchronous status path,
   including the actual PTT output state.
3. Bound messages to negotiated GATT/ATT limits and reject malformed, stale, duplicate,
   or wrong-session requests according to the v0 protocol.
4. Treat control disconnect, lease expiry, reset, and session replacement as receive-
   safe events even if USB audio remains active.
5. Treat USB detach or audio-session loss as an audio fault and a transmit-release
   event even if BLE remains connected.
6. Do not stream audio over GATT and do not embed KX2 connector behavior in the host
   transport.

The protocol issue will decide characteristic layout, payload encoding, request
identity, ordering, errors, security policy, and exact lease semantics.

## Distribution and platform result

| Path | Development-device possibility | Ordinary App Store application | Result |
| --- | --- | --- | --- |
| USB Audio Class | Public iOS audio route; class-compliant hardware needs no app USB driver | Yes, through AVFAudio and the system audio stack | Selected for audio |
| Custom BLE GATT | Public Core Bluetooth central APIs; usage description and, when justified, background mode are required | Yes; no restricted entitlement or MFi dependency documented | Selected for control |
| Generic USB serial | No public iPhone API exposes arbitrary CDC ACM or vendor serial endpoints | No documented ordinarily distributable iPhone path | Rejected |
| Composite USB Audio + serial | Audio function can bind to the system audio stack; the serial function still lacks an iPhone app API | Audio only; composite enumeration does not make serial app-accessible | Rejected |
| Apple Accessory Access | Current beta framework grants exclusive USB access on macOS, not iPhone | No iOS path | Rejected |
| External Accessory | Development and distribution require an MFi accessory protocol authorized for the app | Possible only with MFi participation and accessory authorization | Rejected for the open feasibility baseline |
| DriverKit USB access | Available on supported Macs and M-series iPads with Apple-granted DriverKit entitlements | Not available on iPhone and not an entitlement-free path | Rejected |
| USB Audio + USB MIDI | Core MIDI exposes bidirectional USB MIDI endpoints; a composite can keep one physical cable | Promising class-based wired fallback, but current USB-C iPhone composite behavior is not yet physically validated | Conditional fallback |
| USB Audio + local IP over USB/Wi-Fi | Public network APIs exist, but interface discovery, permissions, addressing, and another network function add work | Potentially distributable, subject to network permissions and hardware support | Rejected for M1 complexity |

## Engineering comparison

| Option | Reliability and background | Latency and throughput | Power and pairing | Complexity and debugging | Portability and tether |
| --- | --- | --- | --- | --- | --- |
| USB Audio + BLE GATT | USB provides a dedicated media route; BLE reconnect and background delivery require explicit testing; safety rests on device lease expiry | USB is suited to continuous audio; GATT is suited to short control values, not media or hard real-time guarantees | USB-C may supply limited accessory power; BLE needs user permission and may pair depending on security policy | Two state machines and coexistence diagnostics, but both use public high-level APIs | Standard paths on Apple and Android; one cable plus wireless control |
| USB Audio + generic serial | Wired when accessible, but there is no iPhone app endpoint | Ample for CAT/PTT | No radio pairing; one cable | Easy on desktops, blocked on iPhone | Poor iPhone portability; one cable |
| External Accessory/MFi | Purpose-built sessions and background mode, subject to licensed accessory behavior | Adequate for control | One cable or Bluetooth Classic pairing | MFi process, protocol declaration, certification, and app/accessory coordination | Apple-specific licensed path |
| USB Audio + USB MIDI | Wired and potentially deterministic; background/app-lifecycle behavior still needs proof | More than enough for M1 control, with MIDI/SysEx framing overhead | One cable; no BLE pairing | Composite descriptors plus non-musical payload mapping complicate diagnostics | Class-based host support is broad if current-iPhone tests pass |
| USB Audio + network control | Network stack can reconnect, but interface choice and local-network state add failure modes | Ample | Wi-Fi has higher power and setup cost | Addressing, discovery, permissions, and network debugging | Broad APIs; physical topology depends on network transport |

No control transport is treated as hard real time. PTT response and release timing are
M1 measurements against bounds chosen by the safety decision.

## Future-host portability constraint

The first proof remains iOS-only, but implementation decisions preserve a credible
Android path:

- RigTether remains the class-compliant USB Audio peripheral and BLE peripheral/GATT
  server; the phone remains the USB host and BLE central/GATT client.
- M1 should select USB descriptors and PCM formats within Android's documented USB
  Audio Class 1 host-mode subset when those choices also satisfy the iPhone proof.
- A descriptor, format, power, or topology choice outside that Android baseline requires
  an explicit owner decision that records the compatibility cost and a credible
  alternate path before Rev A freezes it.
- GATT payloads respect negotiated ATT limits and cannot depend on Core Bluetooth MTUs,
  callback ordering, state restoration, or background scheduling.
- AVFAudio and Core Bluetooth behavior stays inside the iOS host adapter. Firmware,
  safety semantics, protocol definitions, and test vectors remain host-platform
  independent.

Android USB host/audio support and device behavior vary by product and OEM. These
constraints preserve implementability; they do not establish an Android support claim.
Representative-device evidence is required before Rev A freezes the affected behavior,
and a future Android release requires its own documented compatibility matrix.

## Background and App Store constraints

Core Bluetooth documents central-role background execution and state restoration, but
background scanning and execution differ from foreground behavior. AVFAudio documents
background audio as an explicit application mode. App Review permits background
services only for their intended purposes. Therefore:

- the probe declares only the `audio` and `bluetooth-central` modes it actually uses;
- firmware never depends on the app being scheduled to release PTT;
- the app does not persist or restore a transmit request after launch, reconnect, or
  state restoration; and
- an App Store submission must explain the hardware dependency and background use and
  provide review access or a useful simulation. This architecture removes a restricted
  entitlement dependency but cannot guarantee a future App Review result.

## Fallback and trigger

The first fallback is a single composite USB device with Audio Class plus a
class-compliant bidirectional USB MIDI function carrying the same v0 messages in a
transport adapter. It is not selected now because its current USB-C iPhone enumeration,
full-duplex coexistence, application lifecycle, and non-musical framing have not been
validated.

Run the focused current-device validation before relying on the primary or fallback.
Switch from BLE GATT only if repeatable evidence shows that USB audio and BLE cannot
meet the architecture's connection, coexistence, PTT response/release, or recovery
bounds on the supported iPhone/OS matrix. Adopt USB MIDI only if the same validation
shows public Core MIDI bidirectional I/O and USB Audio operating together with
acceptable detach and background behavior. If both fail, return for an owner decision;
do not silently adopt MFi, a restricted entitlement, or proprietary USB access.

## Evidence

Sources were accessed 2026-07-29. Apple documentation is authoritative for API and
distribution availability; USB-IF and Bluetooth SIG specifications are authoritative
for the transports. Statements about expected engineering tradeoffs are inferences to
be validated on hardware.

| Material claim | Primary source |
| --- | --- |
| iOS represents USB audio as an input/output route and may expose separate USB data sources | [AVAudioSession port types](https://developer.apple.com/documentation/avfaudio/avaudiosession/port), [USB audio data sources](https://developer.apple.com/documentation/avfaudio/avaudiosessionportdescription/datasources) |
| USB-C iPhones support USB audio accessories and can source up to 4.5 W to small USB-PD devices | [Charge and connect with USB-C on iPhone](https://support.apple.com/en-us/105099) |
| Core Bluetooth exposes central discovery, connection, GATT read/write, and notification APIs and requires a Bluetooth usage description | [Core Bluetooth](https://developer.apple.com/documentation/corebluetooth), [central role tasks](https://developer.apple.com/library/archive/documentation/NetworkingInternetWeb/Conceptual/CoreBluetooth_concepts/PerformingCommonCentralRoleTasks/PerformingCommonCentralRoleTasks.html) |
| BLE background modes wake an app for defined events but change scanning/advertising behavior | [Core Bluetooth background processing](https://developer.apple.com/library/archive/documentation/NetworkingInternetWeb/Conceptual/CoreBluetooth_concepts/CoreBluetoothBackgroundProcessingForIOSApps/PerformingTasksWhileYourAppIsInTheBackground.html), [`UIBackgroundModes`](https://developer.apple.com/documentation/bundleresources/information-property-list/uibackgroundmodes) |
| App Store apps must use public APIs and background services only for intended purposes | [App Review Guidelines 2.5.1 and 2.5.4](https://developer.apple.com/app-store/review/guidelines/) |
| External Accessory communicates with MFi accessories and the manufacturer authorizes compatible apps | [External Accessory](https://developer.apple.com/documentation/externalaccessory), [Working with accessories](https://developer.apple.com/accessories/) |
| Accessory Access and its entitlement currently manage USB access on macOS | [Accessory Access](https://developer.apple.com/documentation/accessoryaccess), [Accessory Access entitlement](https://developer.apple.com/documentation/bundleresources/entitlements/com.apple.developer.accessory-access.usb) |
| DriverKit USB support is macOS/M-series iPadOS, entitlement-gated, and USB Serial DriverKit is macOS-only | [Creating drivers for iPadOS](https://developer.apple.com/documentation/driverkit/creating-drivers-for-ipados), [USBSerialDriverKit](https://developer.apple.com/documentation/usbserialdriverkit), [requesting DriverKit entitlements](https://developer.apple.com/documentation/driverkit/requesting-entitlements-for-driverkit-development) |
| GATT defines services, characteristics, reads, writes, notifications, and indications for short attribute data | [Bluetooth SIG GATT specification](https://www.bluetooth.com/wp-content/uploads/Files/Specification/HTML/Core-61/out/en/host/generic-attribute-profile--gatt-.html) |
| USB defines class functions independently inside composite devices, including Audio and MIDI | [USB Audio Device Class 2.0](https://www.usb.org/documents?search=Audio+2.0), [USB MIDI Device Class 2.0](https://www.usb.org/document-library/usb-class-definition-midi-devices-v20) |
| Core MIDI exposes bidirectional USB-addressable endpoints and SysEx/UMP I/O | [Core MIDI MIDI Services](https://developer.apple.com/documentation/coremidi/midi-services) |
| Android has public BLE GATT APIs and class-compliant USB audio host support, supporting future portability but not proving a RigTether combination | [Android BLE overview](https://developer.android.com/develop/connectivity/bluetooth/ble/ble-overview), [Android USB digital audio](https://source.android.com/docs/core/audio/usb) |

### Local SDK inspection

On 2026-07-29, `Xcode 26.6` (build `17F113`) with the iPhoneOS 26.5 SDK contained
AVFAudio, CoreBluetooth, CoreMIDI, and ExternalAccessory frameworks but no
AccessoryAccess framework. The macOS 26.5 SDK also lacked the newly documented beta
AccessoryAccess framework. This read-only inspection corroborates, but does not replace,
Apple's platform documentation and does not claim behavior of unreleased SDKs.

Commands:

```sh
xcodebuild -version
xcrun --sdk iphoneos --show-sdk-path
xcrun --sdk macosx --show-sdk-path
```

## Consequences

- The prototype needs simultaneous USB device audio and BLE peripheral capability, but
  this ADR does not select an MCU, codec, module, or USB topology.
- The iOS probe uses AVFAudio and Core Bluetooth without MFi or a restricted
  entitlement.
- Firmware owns independent audio/control health, GATT session state, PTT lease expiry,
  actual-output reporting, and receive-safe disconnect behavior.
- M1 must measure current-iPhone audio formats, BLE timing/reconnect, RF coexistence,
  background/lock behavior, cable and power behavior, and the USB MIDI fallback.
- USB serial cannot be used as a hidden convenience interface for the iPhone contract;
  desktop-only diagnostics must remain clearly separate.
- macOS and Android remain plausible future hosts because the selected transports are
  standardized, but no compatibility claim exists until tested.
- Before Rev A freezes USB descriptors, power assumptions, or BLE semantics, a bounded
  representative Android test must pass or an owner decision must record the accepted
  incompatibility and alternate path.
