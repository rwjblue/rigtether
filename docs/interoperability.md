# Interoperability targets

## Required targets

| Host | Radio | Stage | Required evidence |
| --- | --- | --- | --- |
| USB-C iPhone (iPhone 15 or later) | Elecraft KX2 | M1 primary | USB Audio Class bidirectional audio, BLE identity and frequency read/set, safe PTT, fault recovery |
| Same validated USB-C iPhone | Elecraft KX3 | M1 validation | Repeat the KX2 contract and document any profile or harness differences |

The first proof targets a currently supported iOS release on a USB-C iPhone. Exact
iPhone model, iOS build, USB cable, audio descriptors and formats, BLE parameters, CAT
rates, and radio firmware versions must be recorded with M1 evidence.

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

## Future candidates

Other Apple hosts, macOS, Android, Windows, Linux, and additional radios are plausible,
but they must not expand the first proof. USB Audio Class and BLE GATT avoid
platform-specific serial drivers and have public host APIs on macOS and Android, but
each host still requires explicit interoperability evidence before a support claim.

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
