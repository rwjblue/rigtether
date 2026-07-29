# Track M1: end-to-end bench proof

## Outcome

Demonstrate bidirectional audio, CAT, deterministic PTT, and receive-safe fault recovery from an iPhone through development hardware to KX2, then validate the same contract with KX3.

## Child work

{{CHILDREN}}

## Dependency order

M1 begins only after owner approval of M0. CAT core and platform selection may proceed in parallel once their dependencies land. The iOS probe, control firmware, and bench interface then converge in the KX2 integration issue. Physical KX2/KX3 fault validation closes the milestone.

## Decomposition boundary

The child contracts are provisional until M0 review. Reassess, merge, split, or close them rather than forcing implementation to match stale assumptions.

## Exit criteria

- [ ] Transcript-tested Elecraft CAT logic exists without requiring a live radio.
- [ ] Bench development hardware and audio strategy are selected with rationale.
- [ ] A minimal iOS probe exercises the accepted audio and control paths.
- [ ] Prototype firmware implements the v0 contract and safety state machine.
- [ ] The bench audio/PTT circuit is documented with measured levels and protection.
- [ ] The KX2 proof meets every M1 product and safety criterion on a dummy load.
- [ ] A human records KX2 fault-recovery and KX3 interoperability evidence.

{{RELATIONSHIPS}}

## Completion evidence

Keep this tracker current as children land. Close it only when every exit criterion has real evidence and downstream readiness has been reassessed.
