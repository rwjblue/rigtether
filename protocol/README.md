# Protocol

The RigTether protocol will describe discovery, version negotiation, capabilities,
radio state, commands, PTT leases, errors, and test vectors independently of one
physical transport or host platform where practical. The first host implementation is
iOS, but protocol definitions and fixtures must remain implementable by a future
Android client without reproducing Core Bluetooth or AVFAudio behavior.

The v0 contract is blocked on the M0 transport, safety, and architecture decisions.
Protocol implementations in this directory are licensed under Apache-2.0.
