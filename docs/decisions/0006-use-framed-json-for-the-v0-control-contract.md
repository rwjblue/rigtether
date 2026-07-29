# 0006 — Use framed JSON for the v0 control contract

- Status: Accepted
- Date: 2026-07-29

## Context

ADR 0003 selected BLE GATT for control and USB Audio Class for media. ADR 0004 fixed
device-owned PTT safety semantics and timing. ADR 0005 assigned host-neutral protocol,
session, radio, audio, and safety ownership. M1 now needs one objectively implementable
wire contract and conformance set without selecting an MCU or importing Apple APIs.

The first contract must favor independent implementation and inspection over compact
production encoding. It must operate at runtime-discovered GATT/ATT sizes and preserve
logical payload meaning if current-device evidence later triggers the USB MIDI
fallback.

## Decision

Adopt the normative [RigTether v0 control protocol](../../protocol/README.md).

Logical v0.0 messages are strict UTF-8 JSON objects with integer-only time and duration
fields. BLE GATT carries them through a fixed 16-byte fragment envelope whose value
limits are selected from runtime peer and adapter limits. The envelope is a BLE
transport concern; the conditional USB MIDI adapter, if adopted later, reuses logical
messages but defines its own framing.

The service exposes read-only hello metadata, acknowledged command writes, indicated
operation responses, and self-contained asynchronous status snapshots. Device
`boot_id`, fresh session and operation identities, exact-byte duplicate caching,
ordered requests, independent health domains, typed CAT, PTT leases, and first-cause
faults have explicit fields and machine-readable vectors.

The device keeps sufficient request identity material and every serialized response
until the negotiated session-operation limit, with a v0.0 minimum of 512. This makes
byte-exact duplicates objectively idempotent without permitting cache eviction or a
digest collision to reinterpret a replay. Session exhaustion replaces the session
receive-safely.

The protocol exposes no raw CAT. Its typed radio operations map exactly to the
source-backed KX2/KX3 allowlist and reject unsupported or keying-capable input before
radio I/O. One dedicated firmware safety service remains the only PTT authority.

## Consequences

- Swift, Kotlin/Java, Rust, Python, and diagnostic tools can consume fixtures without
  platform framework types or a CBOR/protobuf dependency.
- JSON and a 16-byte fragment envelope are less compact than a production binary
  format. Fragmentation and bounded message/session limits make that cost explicit and
  testable for M1.
- Exact duplicates are byte-exact. A client that changes JSON whitespace or key order
  while reusing an operation identity intentionally triggers altered-duplicate safety
  handling.
- Indications and write acknowledgements improve delivery visibility but never extend
  device authority.
- A later production encoding is a major-version decision; it cannot silently change
  v0.0 meaning.
- Pairing/security policy, update transfer, hardware/audio choices, MIDI framing, and
  physical evidence remain deferred to their owning issues.

## Alternatives considered

- **Unfragmented one-characteristic JSON:** rejected because it would embed a fixed or
  minimum MTU assumption and would not define independent response/status roles.
- **CBOR or protobuf immediately:** rejected for M1 because their compactness does not
  yet justify another schema/code-generation or codec dependency. Either remains a
  candidate for a later major version backed by measurements.
- **Transport-specific payloads:** rejected because the conditional USB MIDI fallback
  and future Android host must consume the same protocol semantics.
- **Raw CAT passthrough:** rejected because it defeats the source-backed allowlist and
  the sole hardware-PTT authority.

## Evidence

This decision applies accepted, source-backed inputs from ADRs 0003–0005 and the
[KX2/KX3 interface specification](../elecraft-kx2-kx3-interface.md). It adds no
physical result. The platform-neutral conformance harness and vectors are executable
software evidence only; issues #13 and #18 retain human-required physical evidence.
