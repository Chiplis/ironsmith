# Draw-three / discard-from-that-set bodies

Status: source packet ready for independent review; all authored tests UNRUN. Base ff30f190c72b80b9c212d30056d402e94d4f2c86. No build, test, probe, formatter, corpus execution, generated catalogue, accounting, or remote write is authorized here.

Exact frozen identities: Casting of Bones (5a747256-4215-4334-98ab-0c2e4ed92e47) and Soldevi Sage (1f612df3-53b6-4317-9d63-1f903ee3f0c4), copied from cards-20261003.json.xz. Both already have measured-main compile success but remain original-source-unadmitted. This packet earns zero new measured recovery and zero residual credit; it does not edit source admission accounting.

Source inspection: the shared discard clause shape does not list `one of them`, but that omission is NOT a proven bug. The actual complete-body route already exists in `effect_sentences/verb_handlers/zone_move_verbs.rs::parse_draw`: it consumes the exact `then discard one of them` suffix, wraps the draw in TagAffected using a local helper tag, and filters the one-card discard by that same tag and Zone::Hand. This route is present byte-for-byte in retained measured main 5cc46c1 and this base; the retained current Oracle bodies also equal the frozen bodies. This explains measured compilation without invoking an arbitrary-hand fallback. No grammar expansion or normalization change is needed for these two complete bodies.

Runtime trace: DrawCardsEffect reports original instruction actual-draw receipts (including explicit empty receipts); TaggedEffect::apply_outcome_tags reads instruction_result() and applies exact snapshots; DiscardEffect intersects the tagged set with the current hand. Normal reference tracking of an unwrapped Draw is not this route. Source tests below must validate the explicit wrapper through activation and Aura trigger materialization.

Held siblings: Arm-Mounted Anchor (Pirate discard-unless and equip reduction); Eumidian Wastewaker (multi-player discard-or-sacrifice result count and Encore). No sibling admission follows from this pair.

Review must inspect complete source paths and all authored scenarios before deciding on source admission; execution remains deliberately deferred.

## Source-authored scenario inventory (all UNRUN)

`crates/ironsmith-compiler-runtime/tests/draw_discard_bodies.rs` independently invokes strict direct compilation and strict artifact compilation for each complete body. It rejects parse loss and unimplemented content, validates and JSON-round-trips the artifact, then materializes that artifact. Both routes receive exact frozen mana/type/P/T/body metadata, and assert names, identity fixture keys, color, mana value, types, subtypes, body ability counts, and Aura attachment metadata.

- Soldevi Sage: real public activation and tap payment; two distinct controlled lands with split ownership return to their owners' graveyards; insufficient controlled land, wrong zone/nonland, and summoning-sickness rejection; each of three actual drawn cards can be chosen while old own/opponent hand cards and undrawn library cards remain excluded. Ability has no targets. Source control change or departure after activation preserves the activation controller.
- Sage partial/empty library: actual result set of zero/one/two cards, keeping the old hand untouched except for the actual chosen draw. Optional native draw replacement skips zero through three draws; draw history and library counts remain exact.
- Sage pending discard: native stack-resolution checkpoint restores the entire draw and history while retaining already paid sacrifice/tap costs and stack entry. Cloned native state retries once and discards once.
- Casting of Bones: actual paid Aura cast attaches to an opposing creature, rejects noncreature/player target candidates, and fizzles after its target leaves. Real destroy events and SBA/trigger dispatch test ordinary enchanted death, simultaneous host/Aura destruction, Aura departure and further zone change before trigger resolution, split Aura owner/controller, unrelated creature death, exile rather than death, and Aura leaving before host death.
- Bones pending discard/empty library and native optional replacement partial/zero draws retain the exact draw receipt through the triggered wrapper and source departure.
- Candidate assertions use pre-draw stable identities to identify the actual selected cards, but also reject pre-draw ObjectIds: the legal choices must be the new hand incarnations produced by the draw, not stale library objects. Exact candidate-set size and min/max 1 prevent widening or discarding all three. The one-card case permits the native singleton auto-selection path.

## Timing and bounded claims

There is no priority window between this draw and its following discard. Draw triggers do not resolve there, so these tests do not invent an external action between the two instructions. A replacement payload could itself move an earlier drawn card during a later draw; arbitrary such payloads and leave/reenter identity behavior are not newly certified by this packet. Existing original-instruction result/tag owners were source-traced; no speculative identity-owner rewrite was made.

## Review and compatibility

This packet is fixture, regression source, and documentation only. There is no production descriptor change, codec/schema change, runtime payload change, or cache semantic change, so this packet itself does not require a cache boundary bump. Any subsequent discovered production correction must be reviewed separately for that boundary. The earlier proposed grammar extension was removed after tracing the actual existing specialized draw owner. No compiler/build/test/probe/formatter/corpus/codegen execution was performed; no accounting or remote writes were made. Tests are authored assertions, not passing evidence. Source admission remains a separate independent-review decision and this packet does not promote either identity by itself.
