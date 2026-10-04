# Explicit simultaneous phase exchange

Status: **UNVALIDATED** source implementation; no compilation, tests or corpus
replay were run. This closes the previously recorded source-level Time and Tide
interaction and proposes the final phasing body of The War Doctor. Time and Tide
was already baseline strict-compiled and is a supported-card control, not an
additional recovery identity.

## Ownership and representation

A complete phasing sentence with an explicit leading “Simultaneously” and both
all-object arms is recognized before generic conjunction splitting. It carries
both independent filters in one typed PhaseInAll action. The shared PhaseInEffect
payload adds an optional simultaneous phase-out filter, defaulting to absent for
old artifacts. Lowering resolves both filters and materialization transfers both.
All reference/value visitors retain the second filter; no Oracle wording reaches
the runtime as an instruction or card-name special case.

The runtime selects both original sets without mutating the game. It then calls
the shared phase_simultaneously producer once. That producer owns direct/indirect
attachment handling, held-phase rules, common pre-out source/recipient snapshots,
all status updates, and grouped completed-state notifications. In particular, an
observer which is only phasing in is absent from the outgoing event's pre-state.

The former resolution-wide “just phased in” tag is removed. Two authored
sequential instructions may legitimately phase an incoming permanent out again,
and the newly present observer sees their later event. The renderer likewise no
longer invents simultaneity merely because it sees adjacent phase-in/out effects.
Only the explicit typed payload renders the simultaneous exchange.

## Deferred verification

The prior unignored direct/artifact Time and Tide regression remains active with
a source-repair expectation, not a passing claim. Additional authored tests check:

- Both original set filters survive normal lowering and serialized artifacts.
- Old one-way PhaseInEffect payloads deserialize without the optional field.
- Ordinary sequential instructions allow re-phasing the incoming creature and
  permit a newly present War Doctor to observe the later outgoing batch once.
- The sequential surface never becomes “Simultaneously” in generated text.
- Complete grammar retains controller/type/keyword filters and rejects unknown
  compound tails rather than dropping the simultaneity marker.

Public runtime target: phasing_transition_triggers. Full corpus, all-face,
supported-card and runtime gates remain deferred under the campaign workflow.
