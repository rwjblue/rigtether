# Hardware and transmit safety

RigTether controls a radio transmitter. Safety behavior is part of the product contract,
not a later hardening task.

## Required invariants

The interface returns to receive and releases every transmit-control output after:

- power-on or brownout;
- MCU reset or watchdog expiry;
- firmware crash or assertion failure;
- USB detach or host audio-session loss;
- BLE or other control-link timeout;
- protocol parse error or version mismatch;
- application termination;
- firmware update entry; and
- radio-profile change.

A stale command, buffered packet, reconnect, or restored application state must not
reassert PTT without a new, explicit transmit lease.

## Candidate safety mechanisms

The M0 safety decision must determine which are required:

- normally open PTT switching;
- hardware pull-up or pull-down that guarantees receive during reset;
- bounded transmit leases refreshed by the host;
- independent firmware watchdog;
- physical TX-inhibit switch or removable jumper;
- visible TX indication driven by the actual PTT output state;
- output current limiting and voltage clamping;
- audio muting during profile changes and startup;
- separate CAT PTT and hardware PTT arbitration; and
- a test point or loopback that verifies PTT without transmitting RF.

## Bench rules

- Use a suitable dummy load for transmit-path testing.
- Begin at minimum practical RF power.
- Verify radio power, SWR, and load rating before keying.
- Bound every automated PTT assertion with a short timeout.
- Keep a physical means to remove radio power or disconnect PTT within reach.
- Record instrument model, wiring, firmware, app revision, and observed values.
- Stop on unexpected heating, RF output, current draw, ALC, distortion, or latched PTT.

## Electrical unknowns

Until the Elecraft interface issue lands, microphone bias, audio levels, PTT current,
CAT voltage levels, connector ground relationships, and KX2/KX3 equivalence are
unknown requirements. Do not turn typical values into design limits.

This document is an engineering policy, not a safety certification or operating manual.
