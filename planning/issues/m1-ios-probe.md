# Build the minimal iOS audio and control probe application

## Outcome

Build a deliberately small iOS application that proves the selected audio and control transports and exposes enough diagnostics for end-to-end testing.

## Context

The probe is an engineering instrument, not a digital-mode app. It should make routing, levels, protocol state, PTT leases, errors, and disconnect behavior visible.

## Scope

- Enumerate and display the selected audio route and format.
- Capture receive audio and play bounded test audio with explicit level controls.
- Discover/connect to the RigTether control service and display negotiated capabilities.
- Read/set the simulated or real radio frequency through the v0 contract.
- Acquire, renew, and release a short PTT lease with an obvious emergency stop.
- Record timestamped diagnostics suitable for M1 evidence.
- Test transport/protocol logic against the simulator where possible.

## Non-goals

- FT8, CW decoding, logging, waterfalls, polished design, or App Store release.
- Persisting a transmit request across launch or reconnect.
- Private APIs or entitlements not accepted by M0.

## Acceptance criteria

- [ ] The app proves bidirectional audio against class-compliant loopback hardware or the accepted equivalent.
- [ ] Control operations pass against the protocol simulator.
- [ ] App termination, backgrounding where relevant, disconnect, and reconnect release PTT.
- [ ] Diagnostics identify OS/device, audio format, protocol, and firmware versions.
- [ ] A human test procedure covers the physical phone path.
- [ ] `mise run ci` passes.

{{RELATIONSHIPS}}

## Completion evidence

Record the delivered artifact or decision, source and experiment evidence, change or commit, verification commands and results, documentation updates, and follow-up issues. Update the parent tracking issue after the work lands.
