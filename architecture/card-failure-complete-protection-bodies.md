# Complete protection bodies

Status: **UNVALIDATED** source proposal. No builds, tests, compiler or engine
probes, formatters, or corpus replays were run. Git and frozen JSON/hash
bookkeeping do not establish successful compilation or gameplay.

## Frozen scope and disposition

`fixtures/complete_protection_bodies.json.fixture` preserves each exact complete
Oracle body, identity, type/cost/stat fields and both frozen baseline diagnostics.
The source is `cards-20261003.json.xz`; errors come from the immutable
`baseline-e8740178.snapshot.json.gz`. All eight were baseline parser failures.

| Oracle ID | Card | Proposed complete source ownership |
| --- | --- | --- |
| 20822aa7-5f84-442d-add3-b56167e6eb38 | Guardian of the Guildpact | Existing monocolored filter, all protection consumers |
| da790c72-63e7-4955-9435-d073767f9bb5 | Oversoul of Dusk | Complete three-color serial protection list |
| 4f7e865b-6ab0-4361-b864-63c38cdb2fed | Elite Inquisitor | First strike/vigilance retained; complete Vampire/Werewolf/Zombie list |
| 26f87a39-29ed-4649-b5d8-204f6a40a41d | Earnest Fellowship | Each recipient's current own colors, live continuous grant |
| 39a32629-8b05-4539-9ce7-1a6e467d0064 | Empty-Shrine Kannushi | Current colors among its controller's present permanents, including itself |
| e88b3066-eb10-4ade-8207-af4de40facec | Pledge of Loyalty | Enchant creature, live Aura-controller color population, exact self-Aura retention |
| d3fb83af-8fc4-4f26-b2df-be5dd3ac56b2 | Ronom Hulk | Existing Snow supertype filter; complete existing cumulative-upkeep owner retained |
| 6c728edd-ab67-44ce-b19d-aa98dfce68bb | Katilda, Dawnhart Prime | Werewolf protection, live Human mana grant, mandatory six mana and tap counter activation |

All eight are `complete_source_candidate_unvalidated`, not measured recoveries.
No central source-coverage, version, staged-gate or published-stack file changes.

## Prepared-main port

This isolated source port starts at `03a183a8be25c7fb7ec590fa648ce25adbb97028`
and takes the exact original protection delta through
`1211cdaab73eb3be724e997070e4b3924fc5c3f7`. The prepared damage owner, full
source snapshots and checked execution context remain on the new-main
implementation. The existing central `mana_cost(game)` correction is retained,
not reapplied as an older API shape. No coordinator checkout changes are part
of this port.

Source review found that empty comma/semicolon components could survive a
mixed-keyword fallback, and static grant readers could trim malformed suffixes
before reaching the protection reader. The port validates raw protection lists
at both public keyword readers and before static/semantic/document fast-path
grant parsing. Repeated delimiters, delimiters immediately before periods, and
malformed mixed grant tails are explicit errors.

Quoted resolution grants retain live recipient context. Quoted *continuous*
color-population grants are outside the eight frozen bodies and are explicitly
rejected: their current generic grant representation loses quote provenance,
so admitting them would incorrectly bind the granting permanent's controller.
Quoted activated and triggered abilities retain their existing readers.

## Shared grammar and model

The protection grammar owns each quality's exact token and word boundaries,
including serial commas and repeated `from`, before generic keyword splitting.
It rejects malformed symbols and tails instead of accepting a prefix. Existing
chosen-player/color/type, parity, mana-value, exiled-type, commander-identity and
qualified-filter qualities keep their typed owners. A duplicate first-quality
reader was removed so a complete list cannot silently lose later members.

Monocolored and snow lower to existing unrestricted source filters, not
battlefield-only or creature-only filters. Subtype protection therefore also
applies to noncreature sources with the protected subtype. Color and subtype
lists retain every member. No card-name dispatch is introduced.

`ProtectionFrom::OwnColors` and `ColorsAmong { filter, reference_source }`
carry live meaning. Only continuous static grants bind the latter's exact
source ObjectId, so Pledge reads its Aura's current controller. An intrinsic or
quoted granted rule keeps its recipient's context. A live static Aura grant
disappears normally when the Aura leaves, phases out or loses its ability.
The bounded population surface is `permanents you control`; unsupported
qualifiers remain errors rather than an all-colors approximation.

An unquoted resolution instruction instead carries
`ColorsAmongAtResolution(ObjectFilter)`. Its existing resolution-value hook
reads the actual execution controller and checked population once, producing
ordinary fixed `Color` protection before registration. It never needs a live
granting source and cannot borrow the recipient's or a stolen source's
controller. A quoted `This creature has ...` grants a recipient-owned live
static rule; its population continues to track that recipient.

This distinction follows Wizards' [September 25, 2026 Comprehensive Rules](https://media.wizards.com/2026/downloads/MagicCompRules%2020260925.pdf):
CR 109.5 assigns static and activated abilities their respective controller
contexts; CR 608.2h/611.2d determine requested resolution information when the
effect is applied; CR 611.3a keeps static effects current. No fallback-only
controller is retained. The authored synthetic resolution/quoted cases verify
these supported owners without claiming arbitrary quoted or population rules.

Grammar, semantic keyword actions and all static materialization boundaries
carry the typed variants. The existing static artifact/native encoder uses their
canonical payloads. Text changes recurse into the population filter and leave
the abstract own-colors concept unchanged. The protection-from-color selector
recognizes the new color-relative abilities.

These appended payload variants and the shared matcher changes require the
coordinator's later artifact/native/public-digest/audit compatibility gate. This
branch deliberately does not change published version files or manufacture
validation evidence. `ProtectionFrom` appends `OwnColors`, `ColorsAmong` and
`ColorsAmongAtResolution` after `Everything` (zero-based derived-serde variant
indices 13, 14 and 15); all existing variant positions remain unchanged. The
`ColorsAmong` payload carries its complete filter and optional exact
`reference_source`: `None` means intrinsic/recipient context and `Some(id)`
means the exact live static-grant source. No existing serialized field acquires
a new default. The two semantic-only `KeywordAction` additions are also
appended. The next bounded gate must cover artifact/native round trips,
bound-source identity, all three new variants, and canonical public digest/audit
compatibility; older payloads must retain their prior interpretation.

## Rules ownership and checked boundaries

Damage prevention, targeting, blocking, new Aura/Equipment attachment and
attachment SBAs use the same target-relative protection matcher. Blocking's
previous duplicate switch is removed. Sources use current characteristics in
their actual zone, or their exact retained snapshot after leaving/phasing out.
Live population queries exclude phased-out objects and use checked current
colors/controllers. Discovery errors remain on the checked owner's incomplete
execution latch; a false boolean cannot publish a successful action after an
unavailable characteristic query. Missing or unrelated damage-source evidence
is an explicit incomplete-evidence error when the quality requires it.

Pledge's printed exception remains the existing attachment-retention owner.
It exempts only protection granted by the exempting Aura, only from that Aura's
SBA removal. It does not exempt damage, targeting, blockers, new attachments,
other matching Auras or Equipment, or independent protection from another
source. The existing counterfactual calculation excludes the exempting grant
and checks every remaining protection ability.

Katilda's quoted ability reuses the typed source-only selector and existing
`AddOneManaOfAnyColorAmong` instruction. The recipient is the mana ability's
source. Both actual execution and shared mana-production resolution read its
checked current colors, or exact source LKI when required. Colorless produces
zero mana. The native cost/activation owner retains tapping, summoning sickness,
recipient-controller mana credit, no-stack mana execution and real payment for
the separate counter ability. Existing cumulative upkeep adds the age counter,
offers the whole accumulated mana payment, and sacrifices Ronom Hulk on decline
or inability to pay.

## Authored regression evidence, not executed

`complete_protection_bodies.rs` independently compiles all exact complete bodies,
round-trips compiled artifacts, freshly encodes native executable definitions,
and repeats behavioral scenarios through all three routes. Scenarios cover the
static quality matrix across all protection consumers; layer changes; phased
and departed damage-source LKI; live recipient/population colors and controllers;
Aura-only retention versus targeting/damage and other attachments; native bound
static-source retention; grant phasing and ability loss; real cumulative
upkeeps; malformed full bodies; absent/wrong source evidence rollback; and
continuous-discovery failure rollback/recovery.

`katilda_mana_colors.rs` repeats full-body direct/artifact/native paths, current
recipient colors, zero colorless output, legal-action mana activation, tap and
summoning sickness, live grant type/subtype/controller qualification, six-mana
paid counter resolution, phased/departed source-color LKI and missing evidence.
Grammar regressions cover complete serial lists, live scope/retention shapes and
malformed tails, separators and hidden symbols. Protection-led keyword lines
own rejection before generic splitting; public direct and artifact negatives
include trailing comma/semicolon and serial-list tails, alongside supported
mixed protection/flying positives. Actual activated stack scenarios contrast
unquoted resolution specialization with quoted recipient-owned rules through
departed, stolen, phased and present sources, changed recipient control and
post-resolution population/color changes.

The prepared-main base already contains the centrally reviewed ObjectSubject
mana-cost API correction; both protection mana-value callers retain game
context. Additional authored scenarios cover Pledge reattachment and loss of
its exact granting source, the first unaffordable cumulative upkeep after two
successful payments, unrelated source snapshots for Katilda mana, and checked
mana-discovery failure with sequence rollback. These remain unrun.

No remaining source-body gap is presently identified for these eight bounded
candidates. Independent source review, the deferred compilation/runtime checks,
semantic comparison and the frozen full-corpus replay are still required. This
proposal makes no coverage claim for arbitrary color populations, unrelated
protection rules, or any other card identity.
