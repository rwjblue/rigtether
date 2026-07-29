# Interoperability targets

## Required targets

| Host | Radio | Stage | Required evidence |
| --- | --- | --- | --- |
| Current supported iPhone hardware | Elecraft KX2 | M1 primary | Bidirectional audio, identity, frequency read/set, safe PTT, fault recovery |
| Current supported iPhone hardware | Elecraft KX3 | M1 validation | Repeat the KX2 contract and document any profile or harness differences |

Exact iOS versions, connector generations, audio formats, CAT rates, and radio firmware
versions remain to be recorded by M0 research and M1 validation.

## Future candidates

Other Apple hosts, macOS, Android, Windows, Linux, and additional radios are plausible,
but they must not expand the first proof. The architecture should avoid unnecessary
barriers without claiming untested compatibility.

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
