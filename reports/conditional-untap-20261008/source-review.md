# Conditional untap document ownership repair

Base: `1dd81cd84c62f272479f26e16d74719fff24b97b`.
Status: source-authored and statically reviewed only. No build, test, compiler probe, code generation, or corpus execution was performed. `git diff --check` was clean. No remote writes.

## Exact measured regressions

- Send to Sleep: `9e3fd1e9-7db6-40de-b1de-cd8cc9f60590`
- Icy Blast: `f49302c5-8510-4360-841c-a59f53f87e0b`

The fresh baseline report records both as parser failures with `cant-effect > document-reading`: the complete leading condition was passed to the bare restriction parser after the nested untap duration was extracted. The same statement-level registry already excludes leading `if`/`unless`; the document-level registry did not.

## Implementation and compatibility

`effect_sentences/document_readings.rs::read_cant_effect` now declines leading `if` and `unless`, leaving complete predicate/consequence ownership with existing conditional readers. This is a grammatical ownership correction, not a card-name exception, registry-order change, or suppression of parser errors. Malformed conditional bodies remain subject to their owning parser's errors.

The low-level restriction grammar is deliberately unchanged: it documents specialized leading state-condition restrictions (including Demonfire). Broadly banning all conditionals there would change that API unnecessarily. Lowering, runtime, artifact schema, semantic scoring, strict policy, parse-loss policy, source snapshots, generated catalogue, and admission gates are unchanged. Previously generated artifacts are not regenerated or newly credited.

## Authored, unrun coverage

- Grammar: both exact state-predicate forms retain a conditional AST wrapping one prior-object-bound next-controller-untap restriction. Invalid predicate, multi-step duration, and trailing-junk negatives remain errors.
- Whole bodies: exact IDs, full Oracle text, mana costs and instant type in frozen fixtures; independent direct compilation and artifact JSON round trip/materialization; no-loss and no-unimplemented checks; lowering retains one conditional and one tagged untap prohibition beneath it.
- Runtime scenarios: both cards through both routes, zero/one/two targets and matching X, legal/partial/all-illegal selections, true/false predicate at cast and resolution, predicate changes after resolution, separate opponents' next and following untaps, and unrelated late entrants. These are authored expectations, not observed gameplay results.
- Strict baker: full bodies reach unchanged strict compile policy, validate artifact, remain without a semantic score, and contain no unimplemented content. This test does not establish strict corpus admission.
- Whole-body negatives on both routes: next two untap steps, trailing unsupported sentence, and incorrect `until` lifetime must error or record loss.
- Existing standalone restrictions and outer-action/relative-control/quoted-negation tests remain in place and unchanged.

## Separate held issue

Sinister Concierge (`9b14c20f-c4ee-42ea-99cd-099b4eb25883`) is not addressed. Its `that doesn't have suspend` relative clause is classified as a main restriction by `typed_clause_heads`; the negation scanner excludes control/own qualifiers but not this possession relative. This differs from the document conditional boundary. A complete grant/filter/binding implementation and full-body evidence have not been established here. No support credit is claimed.

## Verification still needed when execution is authorized

Run the grammar negated-untap routing tests, compiler-runtime `plural_controller_untap` suite, and baker conditional-untap test; then refresh strict whole-body evidence for the two IDs. Source review alone establishes neither test compilation nor parser, lowering, gameplay, or catalogue success.

## Independent source-review follow-up

Additional authored, still-unrun dual-route whole-body gameplay expectations distinguish predicate semantics and target-domain boundaries:

- Send to Sleep: one instant or one sorcery false; two instants or two sorceries true; mixed instant/sorcery true; two nonqualifying cards false; one qualifying plus a land false; only an opponent's qualifying graveyard false.
- Icy Blast: your power-3 creature false; opponent-only power-4 false; your power-4 and power-5 true.
- Both cards: own and opposing creatures appear in legal targets; noncreature does not. Existing unselected creatures controlled by two opponents are neither tapped nor frozen; independent tapping after resolution detects an overly broad captured restriction set. Selected own/opponent creatures remain tapped through their first respective untaps and untap at their second.

These assertions add discriminating source evidence. They do not change implementation or establish executed outcomes.

The follow-up also authors Icy Blast X=3 with three legal selected creatures across all three controllers, on both direct and artifact routes. It checks exact three-target cardinality, all three taps, and each object's first/second untap boundary, distinguishing dynamic X from Send to Sleep's fixed cap of two. UNRUN.

Second independent-review follow-up adds the two remaining discriminating edges, still entirely UNRUN: Send to Sleep's own qualifying cards in hand or exile (empty own graveyard) do not freeze; the entire zero/partial/all-illegal target matrix now includes an always-unselected existing opponent creature, verifies it is not tapped on resolution, taps it independently, and verifies its normal first untap. Production code remains unchanged.
