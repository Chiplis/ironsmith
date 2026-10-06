# Native keyword token names: source repair, unvalidated

Base: `4e0d4405ca4571840dae46f496e8d890ad189783`.
No build, test, formatter, compiler probe, engine probe or corpus run was
performed. This repair does not alter campaign coverage, published-stack data,
artifact/payload versions or staged gates. All admission decisions remain with
the independent source review.

## Concrete defect and owners

The native Investigate owner bypassed compiler token-profile lowering. It passed
a blueprint named `Clue` into CreateTokenEffect without retained roles. The
native no-Army Amass branch similarly constructed `{subtype} Army` using
Subtype's presentation formatter. The five current token-producing candidates
therefore did not satisfy the existing CR 111.4 naming contract, despite their
parser and effect routes being present.

The shared native Clue blueprint now has the canonical name `Clue Token`, while
retaining its artifact/Clue types and exact {2}, sacrifice-self draw ability.
Investigate supplies rules-implied color/subtype/ability roles and a
SubtypeDerived name role to the existing CreateTokenEffect transaction.

Amass derives its new token's name from the canonical typed subtype vocabulary,
including the `Token` suffix, and supplies the same rules-implied creation
profile. Missing canonical subtype evidence returns IncompleteEvidence before
allocation. There is no Display/Debug or card-name dispatch. Existing Armies
are neither renamed nor recreated; their counters, added subtype, choices and
noncopiable continuous characteristics keep their existing owners.

The typed profile contract is described in `token-text-role-boundary.md`.
Its rules reference is the frozen 2026-09-25 Comprehensive Rules, CR 111.4 and
111.10f: <https://media.wizards.com/2026/downloads/MagicCompRules%2020260925.txt>.
The web reader could not retrieve that URL during this repair, so this packet
does not claim a new independent rules-text fetch. The exact requested names
and the already-reviewed canonical vocabulary govern this implementation.

## Current candidates affected

| Oracle ID | Frozen body | Concrete creating route |
| --- | --- | --- |
| 221240c5-3c97-438c-b906-2508eacf190f | Confront the Unknown | Investigate before targeted Clue-count buff |
| ead0820e-fc6b-4ab2-9110-048d6d53985d | Panther Pounce | Targeted player's Investigate before creature followups |
| 55c7ce94-3cd3-42f3-9dd8-0c118fa626c8 | Secrets of the Key | One or two Investigate actions selected by cast origin |
| 5753f4d6-08d0-4e12-85c5-875ee6441626 | Tidings of War | Amass Goblins 1/3 creates Goblin Army Token only with no eligible Army |
| ab41243d-b178-4ebe-a7ac-bea37427be99 | Wojek Investigator | Repeated Investigate from current opponent hand advantage |

Roalesk (`ca4936cf-48d4-4469-9243-8dbf8ec4cdca`) performs Proliferate and is not
an affected token-producing body. Its separate review is not changed here.

## Earlier proposed frozen bodies affected by these owners

The following nine rows were source-proposed in the base coverage file and
contain an executable Investigate or Amass instruction, rather than only token
reminder text. Their complete copied frozen bodies are in
`fixtures/native_keyword_token_owners.json.fixture`. The common Investigate
lowering dispatch builds InvestigateEffect; the Amass dispatch builds
AmassEffect. Their runtime owner paths end at the native factories above.

| Oracle ID | Frozen body | Exact affected instruction and existing scenario source |
| --- | --- | --- |
| 88522a0f-5377-4522-97f4-4148bef954af | Bolg of the North | Reflexive excess-damage Amass Goblins X; `excess_damage_values.rs` |
| c78d4cec-6764-4cce-96d2-a2d85a57b218 | Eliminate the Impossible | Leading Investigate; `suspected_designation_bodies.rs` |
| a3468804-26ad-4919-b030-a1906a7029e8 | Evidence Examiner | Collect-evidence trigger investigates; complete fixture `collect_evidence.json.fixture` |
| 33c0a8c0-d1d9-4b03-869d-65dab8ace3df | Fall of Cair Andros | Excess noncombat damage trigger Amass Orcs X; `excess_damage_values.rs` |
| c293f1c0-93de-4a73-aaf6-4e9f01e557ec | Innocent Bystander | At-least-three damage trigger investigates; `passive_damage_recipients.rs` |
| db07a3f9-c6d1-4cae-a899-12b3e8d94018 | Inquisitor Eisenhorn | Combat-player-damage trigger investigates that many times; `first_draw_reveals.rs` |
| ec8a4b26-633a-4e88-aa8a-82e9704b3439 | Resonance Technician | Optional ETB discard followed by investigate twice; complete fixture `x_tap_costs.json.fixture` |
| b3a03768-7676-4b33-a813-cc69dea77c4d | The Fugitive Doctor | ETB Investigate; `next_play_and_flashback.rs` |
| e7504407-bb72-4baf-be4a-8152e4198e2b | Thorough Investigation | Controller attack trigger investigates; complete fixture `venture_dispatch.json.fixture` |

These are fourteen exact native-name obligations, not fourteen newly validated
recoveries. This scope does not infer a defect or credit for every token clause.
Inquisitor Eisenhorn's explicitly named Cherubael creation is a separate owner
and is not renamed. Compiler-built ordinary/predefined descriptions, copied
tokens, explicitly named tokens and other keyword-built templates are likewise
not credited by this native-owner patch.

## Authored, unrun evidence

`keyword_action_bodies.rs` independently constructs direct, artifact JSON and
native-definition JSON routes. Its five current full-body scenarios now inspect
canonical names and real Clue characteristics in addition to existing values,
conditions, targets and tails. New native-effect JSON roundtrips execute both
keywords; Clue activation actually pays two mana, sacrifices the selected Clue
and draws for its controller. The existing-Army scenario asserts its original
name remains unchanged. Pending/error replay scenarios retain these checks.

Existing full-body scenarios for Bolg, Fall, Eliminate, Eisenhorn, Bystander and
The Fugitive Doctor now assert the corresponding exact native-created names.
Added complete-body scenarios cover Evidence Examiner's optional real
collection and wrong-turn negative, Resonance Technician's accepted/declined
ETB discard, and Thorough Investigation's real attack declaration plus actual
Clue sacrifice/venture, including an opponent-attack negative. A structural
route test checks all nine full frozen definitions for the concrete keyword
owner on direct/artifact/native routes. This route test supplements gameplay
scenarios; it is not itself full-body gameplay proof.

The engine's native Amass witnesses assert Goblin/Orc/Zombie Army Token names,
0/0 plus real counters, black color, no invented abilities, unchanged existing
Army name and allocation rollback for missing canonical subtype evidence.
Existing Investigate witnesses assert the real Clue Token and its ability.
Older witnesses that observed the same native Investigate route were corrected;
the explicit named `Clue` token-copy fixture was deliberately retained.


## Current prepared-main disposition

The bounded corrections and their prepared-main port are source-reviewed and
admitted as unvalidated proposals in `card-failure-stage97-source-admission.md`.
That record supersedes the earlier pending-admission wording above. The complete
frozen body fixtures and all authored scenarios remain unrun; measured recovery
is unchanged. The coordinated source boundary is artifact9 / digest5 / audit22.
