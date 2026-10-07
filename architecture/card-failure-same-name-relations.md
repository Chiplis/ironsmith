# Same-name relation bodies: source-reviewed proposals, UNVALIDATED

This isolated change is based on `a07ead1cbf8a871568a0aec3ed4715394904c2d9`.
Independent whole-body source review cleared all nine frozen semantic-output
failure identities at `a7ba1a14ebde67a8389c39f5e5fc164fa650cdf5`. These are source
coverage proposals, not measured compile or runtime recoveries. No build, compiler probe, formatter, corpus execution,
or test has run for this change. Diff whitespace inspection is not validation.

The exact Oracle identities and complete printed bodies are retained in
`fixtures/same_name_relations.json.fixture`, extracted without alteration from
`fixtures/card-failure-campaign/cards-20261003.json.xz`. The frozen diagnostic
for each is `compiled text dropped required semantic marker: same-name`.
Semantic validation and the frozen matrix are unchanged. Legions to Ashes is
not included: its independent parser failure is outside this nine-card claim.

## Shared owners

The ordinary terminal name-filter reader now splits the selected set from its
reference before parsing either noun. `the exiled card` denotes the source's
exact linked exile set; `the chosen land` denotes the existing choice result;
`this creature` denotes the source. The search reader consumes `which have the
same name as` and reuses the chosen object instead of asking for another choice.
A Land antecedent surface is appended to the existing typed surface enum.

Live negative comparison sets use `ObjectCharacteristicRelation::SharesNone`
and retain the candidate and comparison qualifiers independently. Another
excludes the candidate's exact incarnation; it does not exclude the source.
A bare token comparison is explicitly battlefield scoped. No new executable
model variant, arbitrary string protocol, or card-name dispatch is introduced.

Current name comparisons use checked characteristic evidence. Source-linked
names read the current exact exile identities, and source-object comparisons
read the current source or its retained departure/phasing snapshot. Arbitrary
result tags retain their captured names. A failed characteristic frame is
recorded on the checked owner's existing failure channel, rather than falling
back to the printed name or satisfying a negated predicate. Name-changing layer
dependencies include tagged name relations. Nameless objects cannot regain a
name from retained split-card alternate-face metadata.

The legacy `tagged_object_name_matches_object_set` helper in `condition_eval.rs`
still compares raw candidate names. None of these new paths relies on it:
Winnow uses an ordinary candidate name relation bound by the trailing-condition
owner, and the negative target selectors use live comparison sets. This change
does not claim that unrelated adapter has been repaired.

## Whole-body paths

- Canoptek Wraith keeps unblockability, combat-player-damage triggering, the
  optional combined mana/sacrifice action, one controlled-land choice, a zero-
  to-two basic same-name search, tapped entry, and shuffle. The already-chosen
  land remains the search antecedent after the Wraith is sacrificed.
- Cylian Sunsinger uses the named same-name pump reader, producing a source
  pump and a separate source-excluding set pump. Both use the complete modifier
  tail reader. The source receives one bonus, other recipients use the source's
  resolution-time name/LKI, and the resolved recipient set and duration use the
  existing pump owners.
- Extraplanar Lens keeps optional targeted own-land exile and the any-player
  land mana trigger. Existing `TriggeringEventProduced` mana provenance and
  the triggered player's controller binding own the produced mana type and
  recipient. A same-name land is not itself required to be exiled.
- Invader Parasite keeps its mandatory targeted land exile, independently
  scoped opponent land-entry filter, and damage to that event's player.
- Strata Scythe keeps land search/exile/shuffle, the attached-creature dynamic
  count over all battlefield lands, and the paid equip ability. Exact linked
  membership and current candidate names drive the count.
- Grim Reminder retains its reveal-only library search and shuffle. The
  named quantified-opponent reader builds a per-player cast-snapshot predicate
  with the same-name filter, then that opponent's life loss. Resolved spells
  remain in cast history. The independent graveyard return and own-upkeep
  activation restriction remain part of the complete body.
- The Apprentice's Folly keeps chapters I and II, a nontoken controlled
  creature target with a controlled-token name comparison, the existing copy
  exception owner (nonlegendary, added Reflection type and haste), and chapter
  III's sacrifice of all controlled Reflections.
- Winnow's existence reader produces `ItMatches` over a live name relation
  with candidate exclusion. The existing trailing-condition binder turns this
  into a condition on the declared destroy target. It does not narrow target
  selection and does not borrow the spell's name. The draw follows the gated
  destruction and still occurs when the condition is false.
- Yenna keeps target uniqueness among other controlled permanents, token-copy
  legend removal, and the Aura-result conditional followed by untap and scry.
  A definite `the token` predicate is now recognized as the preceding creation
  result; an indefinite token remains an existential phrase. The existing
  created-result tag transport and sorcery timing restriction remain owners.

## Authored checks, all unrun

`crates/ironsmith-compiler-runtime/tests/same_name_relations.rs` independently
compiles each complete payload through both direct runtime and JSON artifact
roundtrip paths. It asserts semantic rules and authors native casting, payment,
activation, target announcement/revalidation, triggering and resolution
scenarios. The tests cover all nine full bodies and their secondary abilities.
Additional scenarios distinguish captured/current/copied/departed names,
name-less objects, split names, exact exile reentry, opponent scope, phased
comparison lands, resolved recipient sets, failed target resolution, and a
checked resolution failure followed by retry of the unchanged stack receipt.

Local grammar tests inspect the typed relations, bound source reference,
quantified cast history and definite-token result predicate. An engine helper
regression covers nameless split metadata. These are authored expectations,
not observed compiler or runtime results. Later validation must run these
checks and the authoritative full-corpus/face audit after the campaign gate.

## Corrective source review pass

The first review held the family on six concrete boundaries. The corrective
packet adds exact token consumption for terminal references (including search
references), checked cast-history reads with event caster attribution, checked
actual production-event reads shared by execution and mana projection, and
explicit empty/current/departed object-result transport. A created empty result
is retained through instruction fact aggregation and never becomes the copied
Aura donor. A missing exact produced object's characteristics signals incomplete
evidence rather than inventing its identity.

Current derived characteristics now carry their alternate split name explicitly.
Copy, explicit name replacement and face-down layers clear the previous name
set; copied values retain all actual donor names. Snapshots retain the derived
name set, so renamed or copied split references cannot inherit a printed
alternate name, including a copy whose new primary name equals the old primary.

The legacy cast-history metrics now read the exact captured stack snapshot and
the event's caster independently. A queried player with no casts has a known
zero; a relevant event lacking its required snapshot, or carrying a mismatched
identity/zone, is an error. The production-type reader similarly distinguishes a
complete empty event from missing/mismatched event evidence and never substitutes
a later live source. Its native Lens test changes actual produced green mana to
blue with a replacement, then checks the bonus type and recipient.

Additional unrun scenarios cover Grim's failed hidden-zone search followed by
shuffle, cast actor/controller differences, missing/mismatched cast snapshots,
missing/empty/mismatched production events in both execution and projection,
Yenna's prevented creation, no legal Aura attachment, and token departure in an
added replacement program before the conditional. Wraith's atomic compound
optional-payment correction and its adversarial scenarios are a separate
corrective part of the same branch. The reviewer accepted the complete
corrective packet at `a7ba1a14e`, including modified-cost receipts and the
paid/known-empty choice, search, and shuffle path. No execution has occurred.

## Final handoff

All nine fixture identities are source-reviewed as complete bodies at the
implementation checkpoint above. The final documentation commit changes no
implementation. Integrators must retain the newer central Waterbend/counter
payment owners and the new typed May payload default when resolving overlaps.
No source matrix, central checkout, publication, or measured status was changed
by this isolated lane. The authoritative compiler/runtime/corpus gates remain
explicitly deferred under the implementation-first campaign policy.
