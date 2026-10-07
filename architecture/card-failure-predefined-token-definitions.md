# Predefined token definitions: twelve source proposals

Status: **UNVALIDATED**. No compilation, parser probes, builds, or tests were run.
The strict fixture and gameplay scenarios are authored gates for the deferred
campaign validation, not evidence of successful recovery. Measured campaign
counts remain 40 verified and 3,193 unresolved unique identities.

## Rule and payload boundary

The frozen 2026-09-25 Comprehensive Rules define Vibranium (111.10w), Heartwood
(111.10x), and card-name token instructions with no other characteristics
(111.11). Official source:
<https://media.wizards.com/2026/downloads/MagicCompRules%2020260925.txt>.
Rule 205.3g also classifies Heartwood and Vibranium as artifact subtypes.

A complete Winnow token-name leaf selects a typed semantic shape. Lowering
constructs a complete token CardDefinition. The compiled artifact contains the
printed mana cost, types, subtypes, colors, P/T, abilities, costs, restrictions,
and effects. The runtime neither reparses reminder text nor resolves a card
name through a catalog. The four named-card definitions are frozen from their
canonical Oracle rows, rather than substituting similarly shaped vanilla tokens.
The leaf requires complete consumption; arbitrary suffixes are not dropped.

- Heartwood is a red and green Heartwood artifact with a repeatable tap ability
  producing one red or green mana. It has no sacrifice cost or mana cost.
- Vibranium is a colorless Vibranium artifact with indestructible and a tap
  ability producing colorless mana. Its transaction restriction forbids casting
  nonartifact spells, while allowing other payments; that restriction stays
  attached to the floated mana after the producing artifact leaves.
- Gingerbrute retains its {1} cost, Food/Golem types, 1/1 P/T, haste, paid
  haste-only blocking restriction, and {2}/tap/self-sacrifice life ability.
- Mutavault retains its land type, colorless mana ability, and paid 2/2 animation
  with every creature subtype until end of turn while remaining a land.
- Spellgorger Weird retains {2}{R}, red color, 2/2 P/T and the controller-relative
  noncreature-cast counter trigger.
- Tarmogoyf retains {1}{G}, green color, */1+* printed P/T and a real CDA counting
  distinct card types among nontoken cards in every graveyard. The CDA explicitly
  functions in every zone, including when the token leaves the battlefield.

The existing token-creation replacement, checked-resource transaction,
preparation/original/completion, entry-replacement and trigger owners remain
unchanged. No token cap or alternate creation bypass is introduced. New enum
variants are appended. Artifact-family classification now uses the existing
canonical artifact list, also closing its prior disagreement for Vibranium,
Blood, Powerstone, Infinity and Stone.

## Exact frozen candidate bodies

| Candidate | Oracle identity | Secondary body retained and authored gate |
| --- | --- | --- |
| Aerid Konstrari | 172a8f58-c420-4497-b962-c5853f923667 | Flying, enter/death union, paid creation then artifact-count pump, expiry |
| Hungering Puppetbeast | 690ae865-87bd-46b2-8e65-c53bd80c1a18 | Sacrifice another artifact, permanent counter, chosen temporary keyword |
| Tenured Tethermage | 571cb06d-55b5-46e4-9678-1f74aca73068 | Optional land sacrifice, two tapped tokens, two-artifact tap cost and counters |
| Dora Milaje Elite | 406f1982-5d0c-4dba-9010-28b1c6a3602c | First strike, opponent land-count gate, self-sacrifice legendary indestructibility |
| Shuri's Fabricator | 127f6985-c1f4-487f-b5ee-c84524cd6c55 | Two tapped tokens, paid targeted graveyard return, finality, sorcery timing |
| T'Challa, the Black Panther | ae10eb15-28e7-46d1-9a2e-85763d93febf | Enter/attack union, controller-relative artifact-cast mana-value threshold |
| The Great Mound | 6b78e417-44f8-4ab6-9d3c-e71704fc648e | Mana, paid tap/token and paid tap/draw activations |
| Vibranium Mining Mech | 1bea6d02-186e-46d8-b788-d4cc37dbb480 | Trample, enter/attack union, pump and crew with actual tap selection |
| Ginger, Queen of Sweets | 5ea93956-085d-409b-bd95-dd669fa69eeb | Monarchy, each-upkeep condition and self-sacrifice life ability |
| Mutable Explorer | 34b93a66-de1e-4133-a479-e0d90ba94a3a | Changeling and tapped land-token entry |
| Ral and the Implicit Maze | 3affd37a-d4f6-488a-b30f-8e824b640687 | All three Saga chapters: opponent damage, optional discard/exile/play window, token and Saga sacrifice |
| Tarmogoyf Nest | 29fede1b-e379-46d4-93e9-a589c896add6 | Enchant land; land-controller-owned paid tap/token ability |

All twelve full frozen bodies, not excerpts, are in
`fixtures/predefined_token_definitions.json.fixture`, SHA-256
`ede9054cc9130e79a5246686b1abe12e035324eaf72a3e12ff97c9693ef310bc`.
The four corresponding canonical card rows are in
`fixtures/named_token_canonical_cards.json.fixture`, SHA-256
`f6b901f185eb5908001956aae705050a6251a94bbede24b7c2a9190957fd9375`.
Both come from frozen corpus SHA-256
`9915ac0e2ed2c6fa7e6351842666dc024499e6e4f42812f548036f967bec374c`.

`predefined_token_definitions.rs` authors strict parse-loss and artifact
round-trip gates for every complete body. It exercises direct and restored
materialized definitions through actual entry, stack, activation, cast,
attacker-declaration, payment and trigger producers. Tests also inspect the
canonical token characteristics and their real abilities, conditional negatives,
controller scopes, finality's changed destination and expiring effects.

Ellivere / Virtuous Role and Overlord / Everywhere remain outside this proposal;
their independent secondary prerequisites have not been closed by this change.

The independent bounded source review of `0e34ec164` found no additional concrete blocker in the canonical definitions, complete candidate body routes, or existing creation/payment/receipt owners. Twelve exact identities are admitted as source-proposed/unvalidated in stage52. No executed compile or gameplay credit is implied.

## Subsequent bounded naming hold (2026-10-06)

The eight Heartwood/Vibranium rows above are now partial, not counted. Independent
source review at `9638af33` proved their typed Builtin constructors install bare
`Heartwood` and `Vibranium` names. CR 111.10w/x provides no explicit name, so
CR 111.4 requires `Heartwood Token` and `Vibranium Token`. The frozen instructions
also supply no named clause. The four CR 111.11 named-card token rows are unaffected.
A typed predefined-profile correction is in progress and must receive source and
compatibility review before these eight proposals return. This supersedes only
the earlier naming/full-body disposition for those rows; no execution has run.

Primary rules: https://media.wizards.com/2026/downloads/MagicCompRules%2020260925.pdf
(CR 111.4, 111.10w, 111.10x and 111.11).
