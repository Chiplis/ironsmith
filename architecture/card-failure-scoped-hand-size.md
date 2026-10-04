# Scoped hand-size rules (source proposal, UNVALIDATED)

The candidate cohort has five frozen baseline identities: Anvil of Bogardan, Cursed Rack, Folio of Fancies, Midnight Oil and Price of Knowledge. Inspired Idea, The Second Doctor and Marina Vendrell's Grimoire are recorded as partial fixtures: their other duration/permission/compound-body prerequisites are not claimed here.

## Roots

- The complete no-maximum grammar retains You/Any/Opponent/ChosenPlayer instead of collapsing all rules onto the source controller. The indexed static registry admits each accepted lexical head.
- The existing fixed set/increase/reduce grammar now accepts a chosen-player subject, and its native owner uses the exact source's filter context, including chosen player and actual team relationships.
- A separate typed counter-based static hand limit reads named counters on the actual rule source at each update. It does not turn an arbitrary unresolvable value into zero. New payload variants are appended and mapped through the compiled model/interpreter.
- Existing CR 613.11 timestamp ordering is retained for static and spell-created hand rules. A numeric counter change does not give the source a new timestamp. An unlimited hand size remains unlimited when an additive reduction/increase is applied; a later set operation can replace it.

## Rules references

CR 402.2 and 514.1 define hand size and cleanup discards. CR 613.11 applies these rule changes in timestamp order: https://media.wizards.com/2026/downloads/MagicCompRules%2020260925.txt . The official Inspired Idea ruling confirms a reduction has no effect on an unlimited hand size: https://media.wizards.com/2021/downloads/VOW_Release_Notes/EN_MTGVOW_ReleaseNotes_20211021.pdf . The official Necrodominance ruling illustrates timestamp ordering against an unlimited-hand permanent: https://magic.wizards.com/en/news/feature/modern-horizons-3-release-notes .

## Authored, unrun validation

Strict payload/direct/artifact fixtures cover all five complete cards. Runtime scenarios cover actual double-X and tapping costs, separate opponent mill counts, draw-step player provenance, source control/phasing, chosen-opponent persistence, live counter changes and cleanup discard triggers. Additional properties cover actual teammates, unlimited-versus-reduction and later set limits. Negative grammar examples preserve unsupported tails. No tests, compilation or engine probes were executed; full corpus and supported-card regressions remain deferred.
