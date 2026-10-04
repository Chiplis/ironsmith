# Case and Class token-template follow-ups

UNVALIDATED source-only follow-up to the token-template root. Two additional
whole-card source proposals: Case of the Pilfered Proof and Fisher's Talent.
The exact frozen complete bodies and Oracle IDs are retained in
`fixtures/token_template_follow_ups.json.fixture`. No build, compilation, test,
formatter or replay was executed. `git diff --check` is the only executed code
check; measured recovery remains 40 and unresolved unique cards remain 3,193.

## Case of the Pilfered Proof

The shared trigger reader owns the exact `enters or is/are turned face up`
union. It derives the second arm's complete object filter from the parsed entry
arm, preserving Detective type and controller qualifiers without truncating the
subject to the previous four-token prefix heuristic. This bounded reading
accepts ordinary entry arms only; it does not silently transfer an entry-only
cause/origin/turn gate to an unrelated face-up event.

Both arms use the existing typed native event paths: battlefield entry and
`TurnFaceUpEffect`/the face-up special action. The existing `Either` reference
query maps both arms to the triggering object, and the native face-up event's
object ID is that permanent. The counter body therefore names that current
Detective incarnation, never the Case. Solving uses the existing end-step
condition/solve machinery; the template matcher receives its live Solved gate.

Authored direct/restored-artifact scenarios use actual creation and face-up
operations, then the normal trigger queue/stack. They cover both event arms,
wrong controller and non-Detective controls, departure before resolution,
end-step solving with two versus three Detectives, and the subsequent one-Clue
creation addition. One grammar scenario compares complete subject filters and
reference tags, including a subject longer than four words.

## Fisher's Talent

The frozen baseline reaches the level-two Fish replacement before failing.
The level-one body uses existing executable look, optional conditional reveal,
result-conditioned creation and draw operations. The shared modal-result reader
recognizes `you revealed ... this way` as a successful prior result rather than
an unbound predicate; the creation's result gate follows the optional reveal,
and the subsequent draw remains outside that conditional. The new root supplies
both Fish-to-Shark and Shark-to-Octopus replacement definitions. Existing Class
level designations become live conditions on each replacement matcher, allowing
both different identities to apply in succession at level three.

The full frozen fixture is compiled/materialized in an authored real-upkeep
scenario, not replaced by a reduced example. Its matrix covers all three Class
levels, land versus nonland library top, accepting versus declining the reveal,
actual token type/power, and a draw on every branch. This scenario is unrun and
the source proposal must not be reported as measured semantic closure.

## Inherited limit

The engine retains `TOKEN_PER_PLAYER_LIMIT = 500`. This pre-existing resource
cap is unchanged, and the authored bounded scenarios do not establish unbounded
Magic token-creation semantics. Resource-budget policy needs separate explicit
review; a silently truncated count must not be used as evidence of exact rules
behavior for larger creations.
