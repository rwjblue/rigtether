# RigTether M1 firmware core

`rigtether-firmware-core` is the host-neutral, radio-disconnected implementation of
the M1 v0 framing, strict JSON, ordered/idempotent session model, dedicated PTT safety
state machine, exact USB-audio descriptor/conversion boundary, and typed Elecraft CAT
adapter.

It is intentionally a simulator and shared-logic boundary. The nRF5340
hardware-facing application remains C/Zephyr, and no on-device Rust code is placed in
the safety path without later bounded evidence.

The tests load `protocol/vectors/v0.json` directly rather than copying its expected
outcomes. The vector model retains all 512 v0 operation identities without eviction,
returns exact duplicates without repeating state or I/O, uses a virtual monotonic
clock, and keeps every required health domain separate. CAT integration calls only
the typed `rigtether-elecraft-cat` surface; raw and keying-capable operations prove
that the simulator transport attempt count does not change.

Run the repository task:

```sh
mise run firmware:sim
```

Passing tests are software conformance evidence only. They do not establish nRF5340
timing, USB or BLE behavior, electrical PTT release, watchdog release, KX2/KX3
interoperability, current-iPhone behavior, or human-captured radio evidence.
