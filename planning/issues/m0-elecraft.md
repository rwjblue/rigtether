# Document KX2 and KX3 audio, CAT, and PTT interface requirements

## Outcome

Publish a source-backed interface specification for KX2 and KX3 receive audio, transmit audio, CAT, and PTT, plus a bounded list of values that require bench measurement.

## Context

The product depends on three radio connections that may look similar across KX2 and KX3 but must not be treated as electrically identical without evidence. The specification will constrain the bench circuit, firmware, harness, and safety model.

## Scope

- Use current official Elecraft owner, programmer, and accessory documentation as primary sources.
- Document connector type and pinout, signal direction, nominal and maximum levels where specified, impedance, bias, grounding, PTT behavior, serial signaling, baud/configuration requirements, and command subset needed for M1.
- Identify KX2/KX3 differences, firmware-dependent behavior, menu settings, and connector-insertion hazards.
- Distinguish documented values, reasoned implications, and unknown measurements.
- Define a safe bench measurement plan for every material unknown, including instruments, current limiting, dummy-load use, and stop conditions.
- Update interoperability and hardware-safety docs with the resulting requirements, without turning the measurement plan into fabricated evidence.

## Non-goals

- Designing the final harness or PCB.
- Assuming KXUSB circuitry or unpublished Elecraft internals.
- Live on-air transmission.
- Support for other radios.

## Acceptance criteria

- [ ] A KX2/KX3 comparison table covers every interface used by the proposed product.
- [ ] Every numerical limit has a primary source or is explicitly marked unknown.
- [ ] The required M1 CAT command subset and expected responses are listed.
- [ ] A safe, reproducible measurement plan covers unresolved electrical values.
- [ ] Downstream hardware, CAT-core, and validation issues are updated.
- [ ] `mise run ci` passes.

## Implementation discretion

Read-only research is authorized. Any physical measurement remains human-required unless the user explicitly hands off a supervised bench procedure.

{{RELATIONSHIPS}}

## Completion evidence

Record the delivered artifact or decision, source and experiment evidence, change or commit, verification commands and results, documentation updates, and follow-up issues. Update the parent tracking issue after the work lands.
