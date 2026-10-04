# Completed monarch holder changes (source proposal)

Status: UNVALIDATED, zero measured recoveries. Three exact frozen identities:
Custodi Lich, Garland, Royal Kidnapper, and Knights of the Black Rose. Their
complete inputs, Oracle IDs and baseline diagnostics are retained in
`fixtures/monarch_holder_changes.json.fixture`; all remain uncounted pending
independent source review. No builds, compiler calls or tests have run.

The appended PlayerBecomesMonarch trigger consumes a completed designation
receipt with the previous and newly designated player. Its player reference
is the event participant, independent of the current monarch later. The
checked setter emits once for an actual change, never for retaining the same
holder, clearing the designation or an invalid/prohibited candidate.

The setter stages the completed history and refreshes checked characteristics
before matching. Observers are captured before a later Palace Jailer duration
return can introduce a new observer. Native pending entries retain the event;
the existing generic history/program transport guard remains in force.

The original departure/loss APIs now propagate ExecutionError rather than
ignoring checked discovery. Complete simultaneous loss/draw groups exclude
all departing seats when selecting the successor, share the departure action
boundary, and publish the final monarch change only after all departures.
An observer that leaves in that operation cannot observe the later monarch
frame. Single departure, loss receipt/SBA, game draw/win and WASM forfeit
adapters preserve the checked result. Test/benchmark callers were migrated
mechanically. This does not turn a failed computation into an illegal action.

Knights uses a typed historical predicate, backed by a turn-local recorded
holder. Fresh games begin with no monarch; actual turn boundaries record the
holder once, and extra untap steps do not recapture it. Native copies and Grand
Melee lane state preserve this fact. The wire format carries an explicit
NoMonarch/Player scalar for the main turn and each lane. Missing legacy proof,
invalid seats and inconsistent focused-lane evidence reject before restore;
absence is not guessed to mean no monarch.

Full-body scenarios retain Lich's real entry and targeted player's sacrifice
choice; Garland's opponent-targeted entry, recorded holder target filter,
latched monarch duration, +2/+2 stolen-creature anthem and sacrifice prohibition;
and Knights' repeated loss/gain bodies while the original turn-begin fact
remains true. Direct definitions and JSON artifacts are both covered by authored
scenarios. Native cases pin unchanged/clear/invalid changes, simultaneous seat
departures, observer timing before duration returns and turn-boundary history.

The separate reviewed attack pair (Emberwilde Captain / The Spear of Bashenga)
is in ecb678428 + e0f609af7. Fealty and Courts retain unrelated blockers and
are excluded. New player-only matchers need the quantity worker's additive
damage-reference inventory hook migration when that foundation is integrated.

Deferred validation commands (not run):
- cargo test -p ironsmith-compiler-runtime --test monarch_holder_changes
- cargo test -p ironsmith-engine --lib monarch
- cargo test -p ironsmith-compiler-grammar monarch
- focused departure/Grand Melee/WASM checkpoint regressions and final corpus.

Only source inspection, rustfmt syntax parsing and git diff checks were used.

Independent bounded source review cleared the three complete identities through
992618e25 (mandatory-loop retry and native checked eligibility), ff2a971fc
(final simultaneous-departure eligibility using the original turn anchor), and
d80feb7a8 (recorded holder target announcement/revalidation). The exact three
fixture identities are proposed complete; no execution or measured recovery is
claimed. Regression additions include limited-range rollback/retry, disappearing
monarch restrictions in both departure orders, and multiple holder changes
before Garland's target announcement.
