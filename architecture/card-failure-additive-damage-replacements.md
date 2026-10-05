# Additive damage replacement root (in progress, unvalidated)

Six frozen candidate bodies: Fated Firepower, Hawkeye, Young Avenger; Rankle and Torbran; Taii Wakeen, Perfect Shot; The Flame of Keld; Aether Revolt. No source coverage is claimed by this design checkpoint. No builds, compiler probes, tests, or corpus replay.

The fixed additive static owner already exists, while the resolving duration owner currently handles only multipliers. The missing shared surfaces require complete source/recipient/optional repeated-recipient productions with a typed signed bonus. Static fire-counter and power bonuses read the replacement source at application time; resolving X bonuses are captured once at resolution and outlive their source under the established duration owner. These contexts must not be confused with the damage source or event recipient.

Planned implementation reuses the common DamageAmountReplacementMatcher and checked damage-result modification transaction. Existing integer Add payloads remain schema-compatible. A new typed dynamic modifier/retained static payload must preserve late current-value evaluation and propagate missing/overflow evidence, while the resolving addition carrier freezes its amount and controller. Complete grammar must retain combat/noncombat scope, repeated target identity and all tails, including the alternate 'instead it deals ... plus X, where X is ...' order.

Fated's X entry counters, Rankle's full modal combat trigger, and all Flame chapters are independent full-body gates. Taii's exact-toughness completed recipient trigger and Aether's energy-event amount are explicit additional prerequisites, not automatically covered by the replacement amount reader.

## First native prerequisite

A distinct `RegisterDamageAdditionEffect` now captures its scalar amount at resolution and delegates registration to the existing fixed additive matcher and duration owner. It preserves the integer Add schema, frozen controller, noncombat restriction, legal X=0 and checked missing/overflow paths. Native model interpretation, fresh encoding/decoding and text rendering are wired, with authored lifetime/ownership/zero/error/codec scenarios. This checkpoint does not add parser surfaces or any completed card identity; live static quantities and the complete grammar/full-body gates remain outstanding.

## Live additive native owner

Static modifier payloads now carry an optional typed live delta (old fixed payloads default to absent). The generated dynamic modification is evaluated with the registration source/controller at application, under checked characteristic discovery, then normalized to the existing checked signed Add operation. Resolving X registration remains separately captured. Source counter counts reject missing evidence and values outside the scalar domain; they cannot wrap to negative or silently become zero. Native counter/power/negative/overflow and codec scenarios are authored and unrun. No full-body identity is admitted by this prerequisite.

## Complete grammar and five-body proposal (source reviewed)

The complete additive reader retains duration, source/controller, recipient, repeated-recipient and live-versus-captured quantity. Static live expressions and alternate leading-instead/local-X definitions use existing typed value productions; timed instructions lower through a distinct registration AST with value/reference traversal. Player-or-Battle recipients remain distinct from creatures/planeswalkers. No malformed tail or unrelated repeated target is discarded.

Full direct/artifact scenarios are authored for Fated Firepower (real Flash/X entry and live counters), Hawkeye (Reach/live power/noncombat scope), Rankle and Torbran (all keywords, all modes and universal player/Battle bonus), The Flame of Keld (all chapters, red restriction and source-independent duration), and Aether Revolt (actual captured energy receipt and live Revolt). Player-counter amount annotation now names a compatible trigger and reads the retained marker amount through checked scalar admission. Taii remains partial: its separate exact-damage-equals-toughness trigger needs a completed-recipient semantic owner, beyond the new paid-X registration. Independent bounded source review clears these five bodies through093130e1d; they may be proposed/unvalidated after integration. All execution remains deferred.
