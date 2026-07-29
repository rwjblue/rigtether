# RigTether Agent Instructions

jj-commit-default: auto

## Project context

- RigTether is a pre-prototype open hardware and software interface between mobile
  applications and amateur-radio transceivers.
- The first validated target is iPhone plus Elecraft KX2, followed by KX3
  interoperability.
- Android is a planned future host. iOS-first work must not make the hardware,
  firmware, USB descriptors, BLE protocol, or safety semantics depend on Apple-only
  behavior without an explicit owner decision.
- The project must support bidirectional audio, radio control, and fail-safe transmit
  control. A single physical phone tether is a product goal, not an excuse to assume
  a transport before it is proven.
- There is no accepted MCU, audio codec, USB topology, Bluetooth profile, enclosure,
  or production circuit yet.

## Planning

- Evergreen project documentation under `docs/` is the maintained source of truth
  for product scope, safety invariants, architecture, roadmap, and development
  workflow.
- GitHub Issues are the durable source of truth for unfinished work and open
  decisions after the bootstrap manifests have been published.
- `planning/` is only the reproducible seed for the initial issue set. Do not treat it
  as a second issue tracker after publication.
- Do not implement from a local plan until the user explicitly hands off an
  agent-ready issue.
- An explicitly handed-off, agent-ready GitHub issue is approved scope. The label by
  itself is readiness, not authorization.
- Stop and request direction before materially expanding public behavior, hardware
  scope, electrical limits, protocol guarantees, or safety assumptions.
- Promote settled architecture and safety choices to ADRs in `docs/decisions/`.

## Issue workflow

When explicitly handed a GitHub issue:

- Confirm every blocking dependency has landed.
- Replace `agent-ready` with `in-progress` before substantive work.
- Stay within the issue contract and document any assumption that remains unverified.
- Prefer primary manufacturer and platform documentation over blog posts or forum
  summaries. Record source URLs and access dates for research outcomes.
- Land implementation and documentation together when behavior or constraints change.
- Post completion evidence, close the issue, and update its tracking issue.
- Reassess downstream issues after landing. Apply `agent-ready` only when all
  dependencies are closed and the issue remains bounded and objectively verifiable.
- Apply `needs-decision` when work reaches an owner or architecture choice not
  delegated by the issue.
- Treat `human-required` as a hard evidence boundary. Agents may prepare test
  instructions and artifacts but may not invent bench readings, radio behavior,
  credentials, App Store entitlement results, or physical validation.

## Hardware and transmit safety

- Never infer an electrical limit from connector shape, cable color, or another radio
  model. Cite an official specification or mark the value as unknown and schedule a
  measurement.
- Never transmit into an antenna as part of unattended work. Any transmit validation
  requires explicit human supervision, minimum practical power, and a suitable dummy
  load unless the issue states otherwise.
- The system must default to receive on boot, reset, watchdog expiry, host disconnect,
  protocol error, and control-link loss.
- PTT must fail open. No single stale software state may leave the transmitter keyed.
- Treat microphone bias, PTT, CAT signaling, audio grounds, and radio chassis grounds
  as separate interface requirements until measurements justify combining them.
- Preserve test points, current limiting, ESD protection, and a practical TX-inhibit
  path in hardware proposals.
- Do not publish fabrication outputs as buildable until the corresponding milestone
  exit criteria and human validation issue are complete.

## Implementation preferences

- Prefer Rust for reusable protocol logic, simulators, host tooling, and embedded
  firmware when the selected platform supports it without compromising USB audio,
  Bluetooth, or maintainability.
- Use Swift and standard Apple frameworks for iOS code.
- Keep platform APIs behind host adapters. Shared protocol definitions, fixtures, and
  device behavior must remain implementable by a future Android client without
  reproducing Core Bluetooth or AVFAudio semantics.
- Use KiCad source files for released hardware designs.
- Keep radio-specific behavior behind explicit profiles or adapters. The first
  Elecraft implementation may be narrow, but host APIs must not silently encode KX2
  connector details.
- Prefer transcript fixtures and hardware-independent simulators before live-radio
  tests.
- Avoid new dependencies until they serve an accepted architecture or a bounded
  experiment.

## Validation

- Use the repository tasks instead of ad-hoc commands.
- Documentation and planning changes: `mise run check`.
- Before landing any change: `mise run ci`.
- Hardware research must include a source table and an explicit list of unknowns.
- Bench work must include setup, instruments, firmware/app revision, measurements,
  expected limits, observed limits, and photographs or diagrams where useful.
- The working-copy diff must contain no unrelated changes.
