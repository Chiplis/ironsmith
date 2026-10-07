# Craft materials and paid public successors

Status: UNVALIDATED source proposal reconstructed against `b26366f2befef541c12898765c0de034d4b81ad5`. No build, compiler probe, test, formatter, or corpus execution has run. Historical source review does not establish current validation. The old worktree/commits were lost on workspace replacement; retained design notes guided a fresh implementation and review.

## Frozen whole-card candidates

`fixtures/craft_material_costs.json.fixture` contains both complete original faces, printed characteristics and color indicators, Oracle IDs, and exact frozen baseline category/error/content hash. Inputs are the frozen `cards-20261003.json.xz` and `baseline-e8740178.snapshot.json.gz`.

- Kaslem's Stonetree // Kaslem's Strider: `1ac3e4bc-1678-4280-a071-3dbc8ef4a2bf`. Baseline rejects Cave materials. The front looks at six, optionally puts a land tapped, then puts the remainder on the bottom randomly. The back is a green 5/5 artifact creature Golem.
- Visage of Dread // Dread Osseosaur: `7cad43df-31c9-47b5-bc05-4a0f6544b396`. Baseline rejects two-creature materials. The front reveals a targeted opponent's hand and lets its controller choose an artifact/creature card for that opponent to discard. The back has menace and optional mill two on entry or attack.
- Waterlogged Hulk // Watertight Gondola: `30820f71-9fd4-453c-b377-c2e0b01c07a7`. Baseline rejects Island materials. The front taps to mill one. The back is a blue 4/4 Vehicle with vigilance, live descend-eight unblockability, and crew one.

These three bodies remain candidates, not measured recoveries. No campaign matrix or ledger is changed here.

## Shared owners

The named Craft grammar rule owns surface recognition. Fixed numeric count and card type/subtype are typed facts; there is one count rule rather than separate branches for two, six, and so forth. Unsupported compound/qualified material requirements remain errors. Semantic lowering uses the existing `ExileChosen`, Craft action, and self-exile costs.

A single distinct selection spans other battlefield objects controlled by the payer and other graveyard cards owned by the payer. Existing cross-zone materialization, side-effect-free cost legality, and tagged exile-consumer prechecks remain authoritative. Cost choices reject duplicates, invalid candidates, short answers, and oversized answers before ordinary resolution normalization could replace the selection. Pending `CostEffect` execution yields before publishing results or checking an empty provisional selection as underpayment. Existing native transaction, replacement completion, and resource owners retain rollback and replay; no new transaction executor or resource budget is added.

The sibling Waterbend recovery adds `CostContext` reservations and total-cost bridges. This change adds no context fields or projections and leaves those contracts and counter/alternative-payment receipts intact. Existing i64 quantity APIs are untouched.

## Corrected successor boundary

The [September 25, 2026 Comprehensive Rules](https://media.wizards.com/2026/downloads/MagicCompRules%2020260925.txt), rules 702.167a, 400.7j, and 118.11, establish that Craft's return has no Exile-only restriction, a cost can expose its actual public-zone successor to its ability, and replacement-modified payment remains paid. Thus a directly redirected self-exile into a graveyard is a positive return case. Prevention, hidden arrivals, and separate later movement are different cases.

`SOURCE_COST_PUBLIC_ARRIVAL_TAG` records only the exact public successor created by the original self-exile cost action, before replacement additions execute. Prevention and hidden arrivals record an empty set. `push_to_stack` preserves this completed receipt instead of applying generic stable-ID retargeting. Missing evidence raises the existing `IncompleteEvidence` error; a known empty receipt is a valid no-return result. The new well-known key is appended to preserve existing symbol seed order. Craft's exact tagged-object filter can therefore find a redirected public successor while refusing a later incarnation, including one moved by an addition before stack admission. Existing `SOURCE_EXILED_SELF_TAG` consumers and their contracts are preserved.

`MoveToZoneEffect` still owns transformed entry, owner control, back-face replacements, and material-link transfer. The transfer flag works with the explicitly selected receipt. Entering transformed does not perform a separate transform action. Structural Craft rendering recognizes that typed receipt and the complete material predicates; it does not discard extra branch qualifiers.

## Authored scenarios, all unrun

`crates/ironsmith-compiler-runtime/tests/craft_material_costs.rs` independently invokes strict builder-to-runtime compilation and artifact compilation under separate loss capture. The artifact route serializes, decodes, validates, and materializes all six linked faces. Native activation helpers exercise actual legal actions and staged payments.

Coverage includes exact mana totals and material counts; mixed-zone/graveyard-only materials; controlled opponent-owned permanents; source/controller/owner/zone/type/count/timing negatives; duplicate/invalid/short/oversized choices; pending selection; pending replacement and retry; native error rollback of mana/source/materials/life/events/stack/replacements; exact chosen incarnations; public destination replacement, prevention, hidden destination, later re-exile and pre-stack replacement additions; missing versus known-empty receipts, copied activation receipts and native state restore; transformed back-face entry and no transform trigger; material link transfer.

Full secondary bodies are independently asserted: Stonetree land/remainder behavior; Visage selective opponent discard; Osseosaur menace and optional entry/attack mill; Hulk tap-mill; Gondola crew, vigilance, seven/eight permanent threshold, and cleanup. Grammar and renderer tests cover generic numeric counts, subtypes, unsupported clauses, and additional-qualifier rejection.

## Uncounted partials

Other Craft cards are not admitted by this family. Altar of the Wretched, Eye of Ojer Taq, Jade Seedstones, Ore-Rich Stalactite, Paleontologist's Pick-Axe, Saheeli's Lattice, Sunbird Standard, The Enigma Jewel, and Throne of the Grim Captain still require separate specialized retained-material and back-face-consumer closure. Tetzin's six-artifact clause is incidentally recognized, but both triggered bodies and the whole card are uncounted here.

Deferred execution: `cargo test -p ironsmith-compiler-runtime --test craft_material_costs`, local grammar Craft targets, and text `craft_material_surface_tests`, only after the coordinator opens validation. No execution result is claimed.

## Fresh source review

An independent source review cleared these three complete cards through `59a39eb21658a95e0e23994c2012d182680da27e`. It inspected all six faces and secondary bodies, actual public successor capture before additions, immutable stack transfer, exact membership, required-versus-known-empty evidence, owner-controlled transformed entry, material links, copied/restored receipts, independent compilation routes, and append-only symbol seeding. This clearance is source review only. Every authored execution check remains unrun, and measured recovery is unchanged.
