# Oct8 clean-main refresh: exact-ID analysis

Source: 1dd81cd84c62f272479f26e16d74719fff24b97b; tree ebac6ecfc19ceef218323c68f74b7e9b2b80d393.
Snapshot SHA-256: f7ab0ec3bd02283dcc0f723f1526fed3659f5a85ddb6d85e995fbbd0009314e1.
Filtered data SHA-256: bae465b9d536fffa24c656daff5577a87f5a963dcb2160be0c1dc9ba8e225750.

## Measured current results

32,209 compile entries / 32,138 unique Oracle IDs. 1,824 unique IDs fail compilation; 1,925 remain unresolved under the strict, non-lossy, no-unimplemented gate (1,926 entries). Zero unknown identity outcomes.

Entry statuses: 30,377 strict-compiled, 1,825 parse-failed, 7 permissive fallback. Of strict-compiled entries, 94 are lossy and excluded from supported results. Categories distinguish 1,752 parser failures, 69 semantic-output marker rejections, 3 unsupported-mechanic rejections and 1 compiler panic within the parse-failed total.

Against Oct7, 175 unique IDs recovered and 95 failed the gate after previously passing; 1,830 remain unresolved, for a net reduction of 80. The 95 gate regressions are 88 strict-lossy and 7 parser-failed entries. Lossy counts rose from 4 to 94: all original 4 remain; 88 previously supported entries are now lossy and 2 previously failed entries now compile lossily. This does not establish 88 new gameplay bugs: improved loss detection may expose existing parsing problems.

Against frozen original baseline, 1,445 IDs recovered; 1,788 remain unresolved; 137 formerly supported IDs fail the gate; 28,768 remain supported. Oct7 figures (1,994 compile-failing, 2,005 unresolved, 1,379 original recoveries, 151 original regressions) are historical only.

## Input equivalence and coverage

No new or removed IDs, no renamed/alias-scope changes, no Oracle-text or loader-semantic changes. All 32,138 identities are in the unchanged-semantic-input partition. Complete semantic projection SHA-256: 8bcc5e77799e10c86a66efc1719c11645633985f985b5d16cfd5c3c26bb8b517, identical to original and retained Oct7 evidence. Full printing records changed in 32,093 cases outside this projection.

71 reversible alias entries collapse by their unambiguous face Oracle ID only for unique-ID counts. Source contains 33,127 face payloads; 918 supplemental linked/back/adventure/prepared face payloads were not independently executed. Exact alias membership and supplemental face names are retained.

## Suspected rendered-text signals

182 flagged entries (180 strict-compiled, 2 permissive); 49 newly flagged, 1,210 no longer flagged, 133 still flagged relative to retained Oct7 signal inventory. These are rendered-text heuristics, not confirmed gameplay miscompilations or verified gameplay recoveries. Compiler acceptance likewise does not prove gameplay correctness. Runtime scenarios were not executed.

## Diagnostic triage

899 full per-entry diagnostic signatures partition unresolved entries. 911 overlapping diagnostic-cause groups are a separate view; memberships must not be summed as unique cards or treated as independent proven defects.

- 246 unique IDs / 246 entries: parser_failure: parser does not yet support line family: <text> [rule-path=unsupported-line-family]
- 82 unique IDs / 82 entries: lossy_compilation: suffix_object_filter_recovery: parsed <text> as suffix of <text>
- 63 unique IDs / 63 entries: parser_failure: could not find verb in effect clause (clause: <text>; known verbs: add, move, deal, draw, counter, destroy, exile, untap, scry, discard, transform, convert, regenerate, mill, get, reveal, look, lose, gain, put, sacrifice, create, investigate, attach, unattach, remove, return, exchange, become, switch, skip, surveil, shuffle, reorder, pay, detain, goad, suspect, note, end) [rule-path=statement-line > triggered-line]
- 60 unique IDs / 60 entries: parser_failure: could not find verb in effect clause (clause: <text>; known verbs: add, move, deal, draw, counter, destroy, exile, untap, scry, discard, transform, convert, regenerate, mill, get, reveal, look, lose, gain, put, sacrifice, create, investigate, attach, unattach, remove, return, exchange, become, switch, skip, surveil, shuffle, reorder, pay, detain, goad, suspect, note, end)
- 30 unique IDs / 30 entries: parser_failure: unsupported intervening-if predicate in triggered line: <text> [rule-path=statement-line > triggered-line]
- 30 unique IDs / 30 entries: parser_failure: unsupported predicate (predicate: <text>) [rule-path=leading-if-conditional > sentence-reading] [rule-path=statement-line > triggered-line]
- 29 unique IDs / 29 entries: parser_failure: could not find verb in effect clause (clause: <text>; known verbs: add, move, deal, draw, counter, destroy, exile, untap, scry, discard, transform, convert, regenerate, mill, get, reveal, look, lose, gain, put, sacrifice, create, investigate, attach, unattach, remove, return, exchange, become, switch, skip, surveil, shuffle, reorder, pay, detain, goad, suspect, note, end) [rule-path=statement-line > statement-probe]
- 25 unique IDs / 25 entries: parser_failure: could not find verb in effect clause (clause: <text>; known verbs: add, move, deal, draw, counter, destroy, exile, untap, scry, discard, transform, convert, regenerate, mill, get, reveal, look, lose, gain, put, sacrifice, create, investigate, attach, unattach, remove, return, exchange, become, switch, skip, surveil, shuffle, reorder, pay, detain, goad, suspect, note, end) [rule-path=player-may > chain-reading] [rule-path=leading-player-may > sentence-reading] [rule-path=statement-line > triggered-line]
- 21 unique IDs / 21 entries: parser_failure: unsupported predicate (predicate: <text>) [rule-path=leading-if-conditional > sentence-reading] [rule-path=statement-line > statement-probe]
- 20 unique IDs / 20 entries: parser_failure: unsupported predicate (predicate: <text>) [rule-path=conditional-sentence-family > sentence-reading] [rule-path=statement-line]
- 15 unique IDs / 15 entries: parser_failure: could not find verb in effect clause (clause: <text>; known verbs: add, move, deal, draw, counter, destroy, exile, untap, scry, discard, transform, convert, regenerate, mill, get, reveal, look, lose, gain, put, sacrifice, create, investigate, attach, unattach, remove, return, exchange, become, switch, skip, surveil, shuffle, reorder, pay, detain, goad, suspect, note, end) [rule-path=player-may > chain-reading] [rule-path=leading-player-may > sentence-reading]
- 15 unique IDs / 15 entries: parser_failure: unsupported triggered line: <text> [rule-path=statement-line > triggered-line]

## Seven new parse-failure gate regressions

- Arcane Adaptation (2188c03f-11b6-4651-914a-b4fdd59127e3): parser does not yet support line family: 'Creatures you control are the chosen type in addition to their other types. The same is true for creature spells you control and creature cards you own that aren't on the battlefield.' [rule-path=unsupported-line-family]; oracle-only fallback also failed: parser does not yet support line family: 'Creatures you control are the chosen type in addition to their other types. The same is true for creature spells you control and creature cards you own that aren't on the battlefield.' [rule-path=unsupported-line-family]
- Rukarumel, Biologist (84391c32-5bb7-4c36-be50-bbb5f8732156): parser does not yet support line family: 'Slivers you control and nontoken creatures you control are the chosen type in addition to their other creature types. The same is true for creature spells you control and creature cards you own that aren't on the battlefield.' [rule-path=unsupported-line-family]; oracle-only fallback also failed: parser does not yet support line family: 'Slivers you control and nontoken creatures you control are the chosen type in addition to their other creature types. The same is true for creature spells you control and creature cards you own that aren't on the battlefield.' [rule-path=unsupported-line-family]
- Sinister Concierge (9b14c20f-c4ee-42ea-99cd-099b4eb25883): unsupported complete negated restriction clause (clause: 'each card exiled this way that doesnt have suspend gains suspend') [rule-path=statement-line > triggered-line]; oracle-only fallback also failed: unsupported complete negated restriction clause (clause: 'each card exiled this way that doesnt have suspend gains suspend') [rule-path=statement-line > triggered-line]
- Send to Sleep (9e3fd1e9-7db6-40de-b1de-cd8cc9f60590): unsupported restriction clause body (clause: 'if there are two or more instant and/or sorcery cards in your graveyard those creatures don't untap') [rule-path=cant-effect > document-reading] [rule-path=statement-line > statement-probe]; oracle-only fallback also failed: unsupported restriction clause body (clause: 'if there are two or more instant and/or sorcery cards in your graveyard those creatures don't untap') [rule-path=cant-effect > document-reading] [rule-path=statement-line]
- Leyline of Transformation (cfaa13d6-9992-4223-8bd7-c44046505555): parser does not yet support line family: 'Creatures you control are the chosen type in addition to their other types. The same is true for creature spells you control and creature cards you own that aren't on the battlefield.' [rule-path=unsupported-line-family]; oracle-only fallback also failed: parser does not yet support line family: 'Creatures you control are the chosen type in addition to their other types. The same is true for creature spells you control and creature cards you own that aren't on the battlefield.' [rule-path=unsupported-line-family]
- Loathsome Troll (d360ce89-d80d-4be6-be8c-7e7758cd5840): numeric result row has no preceding die-result owner: '1—9 | Put this card on top of your library.'; oracle-only fallback also failed: numeric result row has no preceding die-result owner: '1—9 | Put this card on top of your library.'
- Icy Blast (f49302c5-8510-4360-841c-a59f53f87e0b): unsupported restriction clause body (clause: 'if you control a creature with power 4 or greater those creatures don't untap') [rule-path=cant-effect > document-reading] [rule-path=statement-line > statement-probe]; oracle-only fallback also failed: unsupported restriction clause body (clause: 'if you control a creature with power 4 or greater those creatures don't untap') [rule-path=cant-effect > document-reading] [rule-path=statement-line]

## Evidence and limitations

analysis/ contains exact original and Oct7 identity transitions, every current unresolved entry with raw diagnostics, suspected rendered-text signals, complete overlapping diagnostic causes, input partitions and face coverage. Original/Oct7 evidence archives were SHA-256 verified against their retained manifest before analysis. The Oct7 full snapshot is not retained: prior exact support is reconstructed from the complete unresolved-entry inventory and cross-checked against exact-ID outcomes; no full Oct7 compiled-definition or similarity-score comparison is claimed.

This is offline analysis only. No source changes, compiler re-execution, extra corpus pass, runtime tests or remote writes were performed. Build/run provenance and untouched raw evidence remain in the parent refresh directory.
