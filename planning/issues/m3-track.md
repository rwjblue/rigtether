# Track M3: developer preview

## Outcome

Publish a documented developer preview with stable-enough protocol, Swift package, firmware configuration/update tooling, reference app, and Rev A hardware instructions.

## Child work

{{CHILDREN}}

## Dependency order

This milestone follows a validated Rev A package. Protocol and SDK stabilization must be driven by working hardware and application evidence rather than M0 speculation.

## Decomposition boundary

Create detailed M3 child issues only after M2 yields validated hardware, firmware, and developer feedback targets.

## Exit criteria

- [ ] The protocol has a documented compatibility and capability-negotiation policy.
- [ ] RigTetherKit exposes audio/control integration without leaking Elecraft connector details.
- [ ] Firmware configuration and update recovery are documented and tested.
- [ ] A reference iOS app demonstrates receive audio, bounded transmit audio, CAT, and safe PTT.
- [ ] Developer, hardware, harness, and safety documentation is complete enough for an independent build and integration.
- [ ] A versioned preview release records supported combinations and known limitations.

{{RELATIONSHIPS}}

## Completion evidence

Keep this tracker current as children land. Close it only when every exit criterion has real evidence and downstream readiness has been reassessed.
