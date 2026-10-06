# Paired exile inspection entitlements

Status: source-authored, UNVALIDATED. Base:
8bb80336b949472d67a82c584e48054be4d2c2bd. Kheru Mind-Eater is the only new
full-body candidate in this packet. Nightveil's previous packet remains the
face-up static-permission prerequisite. The other six original neighbors stay
partial. No builds, compilation, compiler probes, engine/corpus execution,
tests or formatters were run; no campaign ledgers or public versions changed.

The frozen Kheru body is read unchanged from
`fixtures/linked_exile_static_permissions.json.fixture`. The combined static
sentence has one source-linked antecedent and two distinct consequences:
current play authority, and a continuing right to inspect that exact exiled
card once a player becomes entitled. The controller's play authority ends or
changes with the current static ability. CR 406.3 inspection does not require
that the entitled player actually looked and survives losing that ability or
source. Leaving exile ends that exact card/viewer entitlement. CR 406.3 also
ends it if the card becomes part of a shuffled face-down pile. No existing
engine owner implements shuffling such exile piles; that separate interaction
remains a held boundary and this packet does not claim to implement it.

The typed static GrantSpec adds `may_look_at_linked_exile`, default false and
omitted from legacy serialization. Its pair/acquisition lookup uses the same
collector as live static play permissions; no second source-union reader was
added. Native inspection scopes with a narrower or unidentified pool predicate
are rejected explicitly rather than granting inspection to unchecked members. Current exact members and beneficiaries are copied into the existing
per-ObjectId/per-PlayerId inspection entitlement map when a paired exile is
committed, and whenever continuous state reconciles control or abilities.
Historical viewers remain separate from active grant enumeration. Changing a
player's controller derives current access through the existing private-view
boundary rather than persisting the controlling player as a new rules viewer.
Native savepoints retain both maps. A source-only pair import cannot invent a
new viewer, but does not erase an already retained per-card entitlement.

Compiler proof now inventories a complete typed hand-choice/exile dataflow:
one private nonsearch choice from its chooser's hand, then face-down exile of
that exact chosen tag, paired with one combined inspection/play static reader
and optional leaf evasion. It also admits a face-down library producer only
with an explicit inspection consumer. Multiple producers, wrappers, unrelated
static scopes and definition-level cost/program scopes remain held. The
Bishop scalar binder is unchanged. Definition hashes are taken after the
producer's semantic policy is finalized.

A separate privacy gap exists in legacy hidden-selection memory: choosing a
hand card records the chooser among earlier-zone viewers, and ordinary exile
can carry those viewers forward. The newly proved paired hand producer sets
an explicit `exclude_prior_zone_viewers` policy so choosing that player's own
hand card does not authorize later inspection of the new face-down exile.
Only its separately active static inspector grants that right. Older effect
constructors retain their previous default; the new false default is omitted
from their serialized shape. Their broader prior-zone knowledge policy is
not claimed as repaired. Both native exile execution branches apply the same
policy and record only successful actual exile receipts as pair members.

The new full-body scenarios cover direct compilation and artifact decoding,
actual combat and damaged-player choices, private/public viewing boundaries,
spell and land play through normal timing and colored payment, existing/new
controllers, ability removal/restoration, phasing, source/card incarnations,
late triggers, separately borrowed producers, native checkpoints, source-only
imports, pending hand choices, replacement-added failures with rollback and
retry, empty hands/replaced destinations, and unchanged legacy wire checksum
shape. All scenarios remain unrun. Kheru must remain partial until independent
source review clears the entire body and the deferred complete-card execution
pass still applies afterward.
