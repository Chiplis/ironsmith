# Loathsome Troll: activation-owned numeric results

Status: independent source review clear at author head `2e67a0c80e8b1c3d87b647ebc97fe4452cc0a835`, atop production patch `7d594d9645f6edd9a83621554d02cc1260c180aa`; baseline `1dd81cd84c62f272479f26e16d74719fff24b97b`. All compiler/runtime tests remain **UNRUN**. Integrated without conflict as `e5eeb6af7` and `4fc1c3a56`.

## Measured problem and bounded fix

The Oct8 audit rejects Loathsome Troll, Oracle ID `d360ce89-d80d-4be6-be8c-7e7758cd5840`, because its numeric row has no preceding die-result owner. Activation ownership probes included the non-resolving graveyard activation restriction, preventing recognition of the actual roll.

The fix removes recognized activation restriction sentences only from a temporary ownership-probe token copy. Original activation cost, complete body, restrictions and result rows remain unchanged for normal lowering. The immediate unconditional roll requirement and Station/labeled-followup ownership boundaries remain intact. No runtime model or wire payload is introduced.

## Authored evidence and independent review

The independent reviewer inspected the exact production patch, grammar tests and full-body direct/artifact/runtime scenarios by source. No blocking source defect was identified. The scenarios cover labeled/unlabeled restricted activations and a separate following ability, rejection of orphan/non-immediate/optional/conditional producers, a single exact die producer shared by three result branches, results 1/9/10/19/20, source-only movement and destination/order/tapped-state distinctions, graveyard-only legality and exact {3}{G} activation payment with insufficient-green rejection.

An initial synthetic 1/1 harness was replaced before final review. The frozen fixture is the complete official selected printing. All three card runtime scenarios now compile the actual printed card independently through direct and serialized/validated artifact routes: printed mana {3}{G}{G}, Creature — Troll, power/toughness 6/2, green, and the unchanged complete Oracle body. Printed mana and activation mana are asserted separately.

The independent reviewer separately authenticated the fixture against the preserved measured-baseline `reports/current-refresh-20261008/data/cards-current.json`: SHA-256 `bae465b9d536fffa24c656daff5577a87f5a963dcb2160be0c1dc9ba8e225750`; exactly one matching printing ID; entire JSON record equality; exact metadata-plus-Oracle text reconstruction. This was offline source-data comparison, not a compiler probe. The fixture's source-file path is relative to that baseline, not the repair worktree.

## Limits

No new compile, corpus, gameplay or measurement result is claimed. Inspected API signatures are source-consistent, not execution-validated. Broader combined activation restrictions, restriction-bearing Station cases and quoted/iterated-roll negatives were not newly exercised; their production ownership paths are unchanged. Source eligibility, artifact admission, combined-stack review and later executable validation remain distinct. The independent review conclusions above were delivered on Oct8 2026 by `review_numeric_owner_patch`; this report preserves them in the cumulative source packet.
