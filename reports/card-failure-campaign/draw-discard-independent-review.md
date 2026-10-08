# Independent review: draw/discard complete bodies

Reviewed commit: `ba62f8286ddf471329985829f949fc1899aaca61` in `ironsmith-draw-discard-bodies`.
Comparison base: `ff30f190c72b80b9c212d30056d402e94d4f2c86`.
Disposition: **bounded source-proposal clear; no concrete source or authored-evidence defect found**. All tests remain **UNRUN**. This is neither a runtime pass nor a measured recovery/admission-accounting change.

## Scope and identity

The diff consists of exactly three added evidence files: `architecture/draw-discard-bodies.md`, `crates/ironsmith-compiler-runtime/tests/draw_discard_bodies.rs`, and `fixtures/draw_discard_bodies.json.fixture`. There is no production, descriptor, schema, codec, catalogue, accounting, or cache change. The reviewed fixture carries only Casting of Bones (`5a747256-4215-4334-98ab-0c2e4ed92e47`) and Soldevi Sage (`1f612df3-53b6-4317-9d63-1f903ee3f0c4`), including their complete supplied bodies and mana/type/P/T metadata. The tests assert the fixture identity keys explicitly. This review inspected those literal fixture bodies; it did not run a corpus extractor or independently re-extract the frozen corpus.

## Existing ownership path

- `zone_move_verbs.rs::parse_draw` (lines 178 onward) recognizes the exact `then discard one of them` suffix before generic draw parsing. It recursively parses the draw, binds a local helper tag through `TagAffected`, and constructs a one-card, nonrandom discard filtered by that tag and `Zone::Hand`. The whole file has no diff against retained measured main `5cc46c1`, so this is an existing complete-body route, not recovery introduced here. The generic discard-clause omission alone is not a bug.
- Lowering in `compile_support/effect_dispatch.rs` maps `TagAffected` to the outcome-only `tag_all` wrapper, suppressing nested automatic tagging while preserving explicit ownership. `effect_model_interpreter.rs` retains the wrapper and `outcome_only` during runtime materialization.
- `DrawCardsEffect` records actual drawn snapshots and exports an original `instruction_result`, including explicit empty draw/result-object receipts. `TaggedEffect::apply_outcome_tags` reads that original result, while tagging-runtime handling clears empty outcome-only sets. This avoids treating replacement observations or an old hand as the original draw result.
- `DiscardEffect` starts from the resolving player's current hand, intersects the filter, clamps the required number to available legal cards, and does nothing for an empty set. For multiple eligible cards it asks an exact one-card choice; the singleton path may select automatically. The authored assertions correctly allow that singleton behavior.

## Complete authored evidence reviewed

Every scenario loops over two definitions returned by independent strict public compiler invocations. The direct route is not reused from the artifact compiler's side product. Both invocations capture parse loss; the artifact is validated, JSON-round-tripped, compared, and then materialized. Each definition rejects unimplemented content and checks mana value, color, type, subtype, body ability count, and relevant P/T or Aura metadata.

Sage uses public legal actions and native announcement/payment continuation. Source inspection of sacrifice selection/payment confirms the controlled battlefield filter and full-cost availability requirement. The positive fixture pays two distinct lands with split ownership, asserts both destination graveyards, a tapped source, one stack entry, and no targets. Negative fixtures exclude insufficient controlled lands, wrong-zone/nonland resources, and a newly summoning-sick source. Controller change and source exile after activation exercise the captured ability controller.

Draw-selection assertions compare the complete legal candidate cardinality against the precomputed actual drawn stable identities, exclude stale pre-draw ObjectIds, require Hand incarnations and exact min/max one, and inspect the selected card's graveyard versus the other drawn cards' hands. The tests vary the selected index across all three drawn cards and retain preexisting own/opponent hand cards and undrawn library cards. Empty and short libraries, plus optional native replacement decisions accepting zero through three skipped draws, check exact draw history and library counts. The optional replacement helper has an existing explicit grammar owner; its `Do not apply` choice description matches the native replacement option owner.

Bones is actually cast from hand with its printed mana paid. It targets and attaches to a creature, checks noncreature/player exclusions, and separately checks that loss of its sole target causes a paid spell to go to the graveyard unattached. Destruction uses the native simultaneous DestroyEffect and queues its returned events, followed by native SBA and trigger dispatch. The engine batches simultaneous destruction zone changes with lookback-source snapshots; SBA dispatch drains the already-occurred events before removing unattached Auras. Authored assertions cover unrelated creature death, enchanted death, simultaneous host/Aura death, split Aura owner/controller, source graveyard then exile before resolution, exile rather than death, and prior Aura departure. They assert exactly one appropriate trigger and the captured controller. Its triggered body also receives empty/partial/skipped-draw and pending-choice evidence.

The pending cases use `DecisionMaker::awaiting_choice`, not a synthetic undo. `resolve_stack_entry_full` owns a pre-resolution native checkpoint and restores it on pending/error. The Sage assertions preserve already-paid tap/sacrifice costs and the stack entry while rolling back hand, library, and draw history, then clone native state and retry exactly once. Bones likewise checks restoration and single retry after source departure.

## Limits and conclusion

There is no invented priority window between draw and discard. The packet correctly does not claim arbitrary replacement payloads that move a prior draw, or leave/reenter identity behavior, are newly certified. Arm-Mounted Anchor and Eumidian Wastewaker remain outside this packet. No runtime, compilation, test, formatter, probe, corpus, code-generation, or remote write was performed in this review. Existing measured compile success is explanatory context only.

The reviewed source ownership and the complete authored-but-UNRUN evidence support this narrowly bounded source proposal for the two bodies. They do not authorize measured recovery credit, sibling promotion, a cache-boundary change, or a claim that these regressions have passed.
