# Complete token-creation replacement templates

UNVALIDATED source work. No build, compilation, test, formatter, or corpus replay
was run. `git diff --check` is the only executed code check. The measured campaign
remains 40 recovered and 3,193 unresolved unique cards.

## Exact proposed scope

Ten new mechanic-source proposals use the frozen complete bodies in
`fixtures/token_template_replacements.json.fixture`: Bilbo, Fellow Conspirator;
Divine Visitation; Donatello, the Brains; Draconic Visitor; Jinnie Fay, Jetmir's
Second; Queen Allenal of Ruadach; Quina, Qu Gourmet; Stridehangar Automaton;
Tippy-Toe, Terrific Partner; and Worldwalker Helm. The fixture also includes
Jolene, the Plunder Queen, jointly source-proposed only with the separate player attack
declaration trigger patch. Count Jolene once, after both patches are integrated.
Jinnie Fay's alternate corpus entry names share one Oracle identity and must not
be counted twice. Corpus SHA-256:
`9915ac0e2ed2c6fa7e6351842666dc024499e6e4f42812f548036f967bec374c`.

Donatello's Partner—Character select is not justified by recognition alone:
`static_abilities/compiler_model.rs` materializes the labeled `PartnerVariant`,
and `ironsmith-wasm/src/wasm_game_impl/pregame.rs` classifies non-ordinary Partner
labels as a distinct `CommanderPairAbility::Variant`. Pair legality requires
matching normalized labels and independently eligible legendary commanders;
ordinary Partner and other variants do not match. An authored full-Oracle
scenario covers both pair orders and those negative controls.

## Shared mechanism

A named active/passive grammar owns one complete replacement sentence. It
separates a once-per-event addition from an addition/substitution per matching
token, distinguishes mandatory coordinated templates from optional alternatives,
and leaves conditions, extra sentences, first-time limits and unknown recipients
unconsumed. Each recipe passes through the existing full token-definition
compiler. The lowerer requires a complete single-token creation template and
rejects unresolved targets, source-chosen characteristics and embedded entry or
cleanup instructions. This is not a card-name table or a keyword marker.

The appended core payload retains complete nested executable token definitions
through normal effect mapping, artifact serialization and materialization. The
native replacement event carries distinct original, predefined and arbitrary
prototype groups. Each subsequent replacement matches all remaining groups,
including templates an earlier replacement added; matched substitutions preserve
unmatched groups. Existing count modifiers, one-of-each replacement and creation
triggers/history use the same groups. Academy Manufactor now substitutes the
original definition too, rather than retaining a custom Food/Clue/Treasure
prototype accidentally.

Cat, Dog and decline choices use the existing affected-player replacement
choice flow. Alternatives have separate selectable IDs but one parent
application key, including regeneration and optional decline, so one choice
cannot repeatedly replace its own output. Independent ability occurrences
retain independent keys. No template is created until that choice is complete;
the original token instruction's rollback also covers later entry choices.

All groups are committed by the original token instruction. They inherit its
tapped/attacking/blocking setup, Incubate counters, delayed cleanup and applicable
post-creation haste, while retaining their new intrinsic characteristics. A
single complete creation event is published after all groups are created;
subtype-filtered triggers count only matching groups and turn history counts
all actual tokens. Replacement-source references in a template's named-creator
CDA are bound to the replacement source, independently of the original effect.
Live host presence/control and Solved/Class conditions gate the native matcher.

Wizards' [Lost Caverns of Ixalan release notes](https://magic.wizards.com/en/news/feature/the-lost-caverns-of-ixalan-release-notes)
explain that replacement-added tokens inherit the creating effect's instructions
(such as tapped/attacking, granted haste and delayed exile). The CR 614.5/616.1
application identity/order machinery remains the normal engine driver.

## Deferred scenarios and explicit partials

Two grammar, twelve direct/restored-artifact runtime, and one commander-pair
scenario are authored and unrun. They cover complete frozen bodies, replacement
versus additions, once-per-event quantities, Cat/Dog/decline and pending rollback,
filtered creation trigger counts, differing replacement orders, original
prototype removal, artifact versus creature scope, intrinsic keywords/anthems,
tap/cleanup/counters, controller changes, source leave/phase-out, real Jolene/
Quina activation payments, subtype substitution chains and live Solved/Class
conditions. These are proposed checks, not passing results.

Case of the Pilfered Proof and Fisher's Talent were partial in the initial
root. The separate [source-only follow-up](card-failure-token-template-follow-ups.md)
records their complete-body review and authored, unrun scenarios. Their two
proposed identities are additional to the ten above and remain unvalidated.

Crafty Cutpurse's temporary controller redirection, Kaya's temporary token
replacement, and Esix/Mirrormind Crown/Moonlit Meditation's first-event optional
copy choices are not silently accepted by this rule. Their lifetime/first-event/
copy-selection semantics remain separate work. Counter replacement cards found
by broad token wording searches (Doc Samson, Lae'zel and Zabaz) are a different
root and are not counted. No additional measured recovery is claimed.

## Required runtime correctness closure

The later source-only token-resource patch removes the silent 500-token clamp
and introduces exact affordable creation plus typed atomic incomplete-execution
errors. See [token resource boundaries](card-failure-token-resource-limits.md).
All affected token proposals still require deferred exact-count validation;
this is not a verified recovery or a claim of unbounded rules support.

The frozen reversible-card Jinnie alias is included with its exact effective
face metadata. It adds a compile entry, not another Oracle identity.
