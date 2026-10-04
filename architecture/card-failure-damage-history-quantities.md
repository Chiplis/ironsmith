# Current-turn damage quantities (source proposal, UNVALIDATED)

No compiler, build, test, replay, or probe has run for this change. The frozen
metadata/oracle fixture records 14 exact baseline identities with complete source
proposals and authored normal compiler-runtime/tools regressions. Impact Resonance
now consumes retained original damage-occurrence totals and announced divisions;
all coverage remains proposed and unvalidated.

## Complete source proposals

- Blazing Effigy: three plus damage from other sources with the exact printed
  name. `other` excludes the exact dying source, not its announced target.
- Burning-Eye Zubera, Rushing-Tide Zubera: cumulative actual damage to the exact
  dying incarnation, including healed damage and excluding prevented damage.
- Whipkeeper: resolution-time cumulative damage to its announced creature.
- Grothama, All-Devouring: each player's damage-time controlled sources summed
  against the departed Grothama. The independent quoted fight grants bind each
  proper-name participant to its granting Grothama, preserving the receiving
  attacker as the other participant. Two simultaneous grantors remain distinct.
- Dragon Cultist: the greatest total dealt by any one source the ability's
  controller controlled at the time of each damage receipt; receipts to different
  recipients and at different times can contribute to that same source total.
- Case of the Burning Masks: distinct source incarnations with positive damage
  while controlled by the relevant player, with the real solve and sacrifice/
  exile/choice/play-permission body retained.
- Faller's Faithful: the destroyed object's exact prior identity; no selected
  optional target does not mean an undamaged object exists.
- Grisly Sigil: exact prior noncombat damage, with real casualty copy/payment and
  sequential copy/original resolution independently rechecking history.
- Sold Out: the exiled creature's old identity and real Clue creation.
- Hawkeye, Avenging Archer: this exact Hawkeye dealt damage to that exact dying
  opposing creature; later incarnations do not inherit the relation.
- Thirsting Axe: the current equipped host, or the exact departed Equipment's
  attachment receipt for its pending trigger; only combat damage to a creature
  satisfies the exception. The consequence sacrifices that host.
- Wolverine, Best There Is: damage to another exact creature, preserving the
  source antecedent of “him”, the real damage multiplier, and regeneration.

## Shared representation and execution

The appended `Value::DamageHistory` contains typed source/recipient scopes,
combat qualification, and total, largest-per-source-total, or distinct-source
reduction. It inspects completed positive DamageEvent receipts. Historical
characteristic scopes use the producer's source/recipient snapshots, never
current-controller/type/name guesses. Explicit references retain ObjectId and
never follow stable card identity. Missing required characteristic receipts and
unbound object references return a value error.

Sums use checked u64 intermediates, comparisons checked i64, and effect scalar
materialization checked i32. This avoids overflow converting a large valid
history amount for a small threshold comparison. It is not arbitrary-precision
rules support; native damage producers still have their existing u32 bounds.

The ordinary selection filter's `other` interpretation (announced target set
and stable-card identity) is deliberately not used by historical classes. Here
it excludes the exact resolving source. The legacy DamageDealtToSource history
query also stops following stable identity, retaining its existing scalar API.

Reference, player-iteration, target-discovery, tag-walking, description, and
continuous turn-context paths traverse the new payload. Pure source queries are
readable by the existing continuous numeric adapter. This change does not admit
new resolution-only values through dynamic anthem capability guards.

Current live attachment state takes precedence over any saved attachment.
Source absence uses the latest true departure receipt and then exact retained
source LKI; a live unattached source never revives an earlier host. The same
helper is consumed by the host condition, value, and attached-object prelude.

## Impact Resonance's original blocking boundary

Its value is the greatest amount dealt by one source to one recipient in one
occurrence, not a source's total this turn. Replacement processing can split an
original damage packet into redirected/remainder assignment receipts, even
returning multiple fragments to the same recipient. A maximum receipt amount
undercounts those cases; grouping by source for the turn incorrectly merges
independent occurrences. An actual producer-level occurrence identity or safe
post-replacement coalescing boundary must be designed before this card can be
counted. The corrections below supply that boundary without substituting a
largest-single-receipt or a whole-turn source total.

## Authored, unrun coverage

Normal runtime suite compares direct and serialized-artifact definitions for
all 14 complete proposals and exercises each complete printed body. Engine
regressions cover actual prevention/regeneration, exact blink identity,
damage-time control, total versus per-source versus source-count reductions,
wide comparisons/explicit scalar overflow, bound prior objects, and execution/
continuous agreement. Grammar tests assert typed shapes and reject attempted,
previous-turn, unsupported-owner and unsupported-occurrence forms. Normal tools
aggregate requires metadata-bearing strict, non-lossy compilation.

## Occurrence-maximum prerequisite (source only)

`LargestSourceRecipientOccurrence` is appended to the history reduction enum.
The completed effect damage owner now binds per-source/per-recipient totals on
its original reported receipts before matching, including when an outer action
holds publication. The queue binder preserves already-completed totals instead
of merging unrelated instructions which share a broader publication batch.
Combat already binds its complete original receipts at its producer boundary.
A grouped historical receipt missing completion evidence fails explicitly.

Authored tests distinguish 2+2 fragments redirected to two recipients (maximum
2) from the same fragments reconverging on one recipient (maximum 4), and two
independent instructions under one outer batch (maximum 3 versus source total
5). The strict full-card count remains 13 until Impact's chosen-distribution
announcement binding and complete allocation-batch owner are corrected.

The official Commander 2014 release notes establish Impact's announcement-time
X/division and preserved shares after partial target illegality:
https://magic.wizards.com/en/news/feature/release-notes-2014-11-10
No tests, builds, compiler probes, or gameplay execution have run.

## Completed allocation owner and Impact closure (source only)

Chosen divided damage now uses the retained announcement's complete allocation
sum as its budget. It does not read a changing or unavailable effect value again.
Illegal targets lose their original shares. Even division retains its separate
existing target-count semantics. All surviving allocations become one complete
native damage batch, so a first recipient's replacement-added instruction cannot
remove a later original or its observers before that original is committed.

The direct executor establishes checked continuous state and shares the existing
resource transaction across source binding, allocation choice, damage originals
and additions. A pause/error restores consumed target distributions as well as
game state and execution context. Allocation arithmetic is checked rather than
wrapping, and an unrepresentable native outcome is a typed incomplete result.

The exact Impact direct/artifact scenario announces 5 after one source dealt 5 to
four recipients, then deals another 8 in response and optionally blinks a target.
Its original 3/2 division stays fixed, the blinked incarnation takes none, and the
remaining target receives only 2. A zero-history/no-target cast is also authored.
Arrow Volley Trap's exact existing fixture now has an alternative-cost casting
scenario with an illegal original target and another with a first-recipient
addition that exiles the second: both allocated original damage receipts must
precede that addition. This repairs its shared-owner hold; no extra Trap count is
claimed. All scenarios remain unrun.
