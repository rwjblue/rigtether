# RigTether Elecraft CAT core

`rigtether-elecraft-cat` is the transport-independent M1 radio-control core. It owns
only the source-backed KX2/KX3 CAT allowlist and deterministic transcript replay. It
does not own serial electrical signaling, BLE or USB framing, iOS APIs, audio, hardware
PTT, leases, or sensed `PTT OUT`.

## Public API and integration

Create a `CatSession` with one explicit `Profile` and an adapter implementing
`RadioIo`. Submit only typed `Operation` values through `Request::Typed`. A transport
adapter receives complete allowlisted ASCII commands and returns incremental response
chunks plus deterministic elapsed time. This keeps serial scheduling outside the core
while testing partial reads and timeouts inside it.

The radio service supplies a `FrequencyPolicy`. The crate checks the eleven-digit
`FA` encoding before I/O and includes `DocumentedFrequencyPolicy` for fixtures. A
production service must add its accepted profile and operator-facing frequency limits;
the crate intentionally does not invent legal bands or transverter configuration.

`Operation::SetFrequency` sends the SET and then queries `FA`. It returns the confirmed
frequency only when the policy accepts the exact value or documented non-FINE 10 Hz
normalization. A SET response alone never completes the operation.

Every `Error` exposes:

- `code()`, which maps to stable v0 `unsupported_radio_operation`,
  `invalid_argument`, or `radio_control_fault`;
- `radio_io_attempted()`, suitable for the v0 `radio_io_attempted` field; and
- `safety_directive()`, which maps post-I/O CAT uncertainty to immediate release and
  lockout without waiting for CAT recovery.

`TxObservation`, `OperatingState::tx_state`, `IF`, and `TQ` are diagnostic
observations only. They cannot grant, renew, or sustain transmit and never replace
radio-side sensed `PTT OUT`.

## Supported commands

The complete surface is:

| Typed operation | Emitted CAT |
| --- | --- |
| Normalize session | `AI0;` then `AI;`, `K20;` then `K2;`, `K30;` then `K3;` |
| Identify | `OM;` |
| Read firmware | `RVM;` and caller-selected optional `RVD;` |
| Read VFO A | `FA;` |
| Set VFO A | `FAxxxxxxxxxxx;`, then `FA;` |
| Read operating state | `IF;` |
| Read mode | `MD;` |
| Read transmit observation | `TQ;` |

The parsers validate the whole semicolon-terminated response, including fixed fields,
model correlation, and the KX2/KX3 mode difference. `FrameDecoder` accepts partial and
concatenated reads. The session reports malformed, unexpected, unsolicited, `?;`,
timeout, model-mismatch, verification-mismatch, and inconsistent `IF`/`TQ` cases as
typed faults.

## Unsupported commands and safety boundary

There is no raw write method. `Request::RawCat` exists only to reject an untrusted raw
request before `RadioIo` is called. `TX`, `RX`, XMIT/TUNE `SWT`/`SWH` emulation, `KY`
or other text/keyer transmission, power/mode/VOX/menu/baud writes, raw passthrough,
and every unlisted operation are rejected with `radio_io_attempted() == false`.

CAT timing never becomes transmit authority. Fixtures record the documented under-10
ms typical case as 5 ms. Ordinary queries tolerate the approximately 100 ms documented
general delay, and frequency sets model the up-to-500 ms band-change case. Provisional
software deadlines add conservative margin (50/250/750 ms respectively) for the
complete response and adapter overhead, without assuming undocumented serial word
framing. They are not physical claims and remain replaceable by issue #13
human-captured timing evidence. These values affect only radio-control scheduling and
faults.

## Simulator

`simulator::Simulator` replays the neutral fixture format described in
[`fixtures/README.md`](fixtures/README.md). It records every attempted command,
advances a deterministic clock, supports response chunking and timeout records, and
can reset to the start of a transcript. The format can be consumed directly by Rust
firmware and host tools, Swift tests, and a future Android adapter without adopting
BLE framing or Apple types.

```rust
use rigtether_elecraft_cat::simulator::{KX2_DOCUMENT_FIXTURE, Simulator};
use rigtether_elecraft_cat::{
    CatSession, DocumentedFrequencyPolicy, Operation, Profile, Request,
};

let simulator = Simulator::from_fixture(KX2_DOCUMENT_FIXTURE).expect("valid fixture");
let mut session = CatSession::new(Profile::Kx2, simulator);
let result = session.execute(
    Request::Typed(Operation::NormalizeSession),
    &DocumentedFrequencyPolicy,
).expect("normalization replay succeeds");
assert_eq!(format!("{result:?}"), "SessionNormalized");
```

The included artifacts are document-derived synthetic transcripts, not observations
from a radio. Serial/electrical integration and human-captured transcripts remain
issue #13 work.
