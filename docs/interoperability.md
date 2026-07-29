# Interoperability targets

## Required targets

| Host | Radio | Stage | Required evidence |
| --- | --- | --- | --- |
| USB-C iPhone (iPhone 15 or later) | Elecraft KX2 | M1 primary | USB Audio Class bidirectional audio, BLE identity and frequency read/set, safe PTT, fault recovery |
| Same validated USB-C iPhone | Elecraft KX3 | M1 validation | Repeat the KX2 contract and document any profile or harness differences |

The first proof targets a currently supported iOS release on a USB-C iPhone. Exact
iPhone model, iOS build, USB cable, audio descriptors and formats, BLE parameters, CAT
rates, and radio firmware versions must be recorded with M1 evidence.

## Radio-profile baseline

The source-backed [KX2/KX3 radio interface specification](elecraft-kx2-kx3-interface.md)
defines the M1 radio-side contract and the measurements that still require a human:

- KX2 and KX3 have separate profiles even where Elecraft documents similar connectors
  or CAT commands.
- Receive audio comes from `PHONES`; transmit audio comes through `MIC`. Their actual
  voltage, impedance, bias, common-mode, and clipping behavior must be measured before
  the adapter circuit is selected.
- CAT uses KX2 `ACC` or KX3 `ACC1` at a 4,800 bit/s baseline. The M1 adapter exposes a
  typed read/frequency-set allowlist, not raw CAT, and cannot emit a transmit command.
- Hardware PTT is the only RigTether-controlled keying path. KX2 uses a measured
  mic-PTT implementation; KX3 requires an explicit choice between measured mic PTT and
  `ACC2 IO LO=PTT`. Exactly one path is enabled per profile.
- KX3's native inhibit is optional defense in depth, not a replacement for the
  independent physical TX inhibit and sensed `PTT OUT` required by ADR 0004.
- Real-radio electrical and insertion evidence belongs to issue #13 and remains
  `human-required`. No KX2/KX3 physical result is claimed by the source study.

## Host transport baseline

- Bidirectional media uses a class-compliant USB Audio function surfaced as the iOS
  `usbAudio` input and output route.
- CAT, capabilities, status, and bounded PTT leases use the RigTether BLE GATT service.
- The app uses public AVFAudio and Core Bluetooth APIs. The baseline does not require
  External Accessory protocols, MFi participation, DriverKit, Accessory Access,
  proprietary USB serial access, or private entitlements.
- USB Audio and BLE must be tested while active together. Validation includes route
  changes, lock/background behavior, BLE interruption, reconnect, USB detach, app
  termination, and wireless interference close to the USB-C cable.
- A connected audio route is not proof of a healthy control link. A connected BLE link
  is not proof that audio is routed correctly.

Older Lightning iPhones, iPads, and wired-control fallbacks are future compatibility
work, not M1 support claims.

## Future hosts

Android is a planned future host, but it must not expand the first iPhone proof. Other
Apple hosts, macOS, Windows, and Linux remain candidates. USB Audio Class and BLE GATT
avoid platform-specific serial drivers and have public host APIs on macOS and Android,
but each host still requires explicit interoperability evidence before a support claim.

The Android preservation baseline is:

- keep the phone in USB host mode and RigTether as a class-compliant USB Audio
  peripheral;
- prefer descriptors and PCM formats inside Android's documented USB Audio Class 1
  host-mode subset when that also satisfies the iPhone proof;
- require an explicit owner decision and a documented alternative before accepting an
  M1 or Rev A choice that falls outside that subset;
- keep the phone as BLE central/GATT client and RigTether as peripheral/GATT server;
- negotiate GATT/ATT sizes and protocol capabilities at runtime rather than encoding
  Core Bluetooth behavior; and
- perform a bounded test on representative USB-C Android hardware before Rev A freezes
  USB descriptors, power assumptions, or BLE behavior.

Android device and OEM variation means this baseline preserves a credible path; it is
not an Android compatibility claim. Official platform baselines are documented in
[Android USB digital audio](https://source.android.com/docs/core/audio/usb) and the
[Android BLE overview](https://developer.android.com/develop/connectivity/bluetooth/ble/ble-overview).

## Compatibility evidence

A support claim requires:

- model and relevant firmware/OS versions;
- harness revision and pinout;
- interface hardware and firmware revision;
- host library or app revision;
- audio format and level settings;
- CAT/control settings;
- PTT mechanism;
- fault tests performed; and
- known limitations.

For KX2/KX3, the evidence must also include every radio-side value and powered
insertion behavior identified as measurement-required by the radio interface
specification.
