# Replacement trigger receipts (UNVALIDATED)

Follow-up to qualified draw/life participants. Wedding Ring remains partial; no new full-card count is claimed by this commit.

## Closed source boundaries

A completed original event must be matched before appended programs can remove a qualifying permanent, change a participant, or change the amount read by a later trigger. Matching now retains the original physical notification in its outcome and attaches an internal capture proof only after ordinary and delayed trigger checks have actually occurred. Later publication still records physical history but cannot discover those same triggers a second time. The proof survives event clones and metadata enrichment; it is private runtime receipt state, not an Oracle marker or serialized core variant.

The shared addition executor captures originals and earlier programs before the next added program. Replacement payloads have an instruction-matching scope of their own, including nested sequences. Effect-backed costs capture before the next cost component while retaining events for payment quantities. Pending/error owners restore their native game/context checkpoints, including deferred entries and capture proofs.

Simultaneous life actions have replacement preparation, original commit, and addition phases. The proposal interface has a default choice-free preparation hook; gain/loss/set-life proposals override it. ForPlayers calls every preparation before any mutation. Combat lifelink uses the same phases across sources. Completed combat life/counter receipts are captured before damage additions and prevention follow-ups. Ordinary exchange-life already supplies its full proposal vector to the same owner.

Authored direct/artifact scenarios cover original draw/life plus artifact removal; nested draw-then-remove payloads; a qualification created too late; effect-backed cost gain; two-source simultaneous combat lifelink; repeat delayed listeners without duplicate publication; and suspended added-program rollback/replay. Native prestate tests use life-dependent replacement eligibility across each-player gain/loss. Shared draw-step coverage uses the real TurnRunner and preserves sequential player draws.

## Remaining concrete gap

CR 121.7 orders unreplaced parts of an event before replacement-created draws. The current generic life original commit still executes an `Instead` payload immediately. Therefore an each-player life instruction with A's gain replaced by a draw can execute that draw (and its added artifact removal) before B's unreplaced gain. That can wrongly remove B's qualifying Wedding Ring before the gain is matched.

`replacement_created_draw_waits_for_other_original_life_changes_before_removing_qualification` is an active, unignored complete-card regression for this gap. The fixture remains partial. A general payload continuation/scheduling boundary is needed; merely deferring every entire Instead program would incorrectly move non-draw original effects too. Do not count grammar acceptance as closure.

Rules source: [official September 25, 2026 Comprehensive Rules](https://media.wizards.com/2026/downloads/MagicCompRules%2020260925.txt), CR 121.2c–d, 121.6b, 121.7, 805.6a. Shared-team draws are sequential, and each draw's replacement completes before the next draw, so the simultaneous-life batching rule must not be imposed on them.

No builds, compilation, compiler probes, or tests were executed. Changed Rust source was parsed by rustfmt and whitespace checked only.
