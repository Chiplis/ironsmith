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
condition/solve machinery. A later source review found that the original
proposal retained the Solved presentation but had no executable static guard.
The shared Case static-condition correction now lowers that typed label to
`SourceCaseSolved` on every static member. Until deferred execution, the earlier
unrun scenario is evidence of intended behavior rather than proof that the
original implementation gated the matcher.

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

## Resource correctness gate

The later source-only token-resource patch removes the silent 500-token clamp
and supplies checked totals and atomic resource-exhaustion errors. See
[token resource boundaries](card-failure-token-resource-limits.md). The Case and
Fisher's Talent proposals still require deferred validation; authored 501+
scenarios do not by themselves prove complete or unbounded execution support.
