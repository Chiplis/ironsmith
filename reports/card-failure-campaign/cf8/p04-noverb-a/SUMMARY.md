# cf8 / p04-noverb-a — summary

Package: 151 cards failing with "could not find verb in effect clause". Branch cf8/p04-noverb-a, based on origin/main 84ea8b41c. Nothing was built or run (campaign policy). The prebuilt hint binary vanished mid-run; the fallback `.agents/target-score/release` binary is from June, so its results are weak evidence.

Status: 32 source-proposed, 118 blocked, 1 untriaged.

## Fixed clusters (source-proposed)
- **elided-damage-recipients** (Tropical Storm, Hail Storm, Neonate's Rush, The Fall of Kroog, Wildfire Howl)
  - The paired damage fanout now reads the second part when it is "N additional damage", a recipient plus an each-set, "its controller", or "that player". It also strips a leading self-replacement "instead".
  - Files: fanout_family.rs, fanout_shapes.rs, back_references.rs, damage.rs.
- **forced-attack-requirements** (Incite War, Instigator, Nettling Curse, Rowan Kenrith)
  - The attack-if-able primitive now accepts `creatures`/`enchanted`/`equipped` as heads.
  - Rowan's "during target player's next turn ... attacks if able" becomes TargetOnly + CantEffect MustAttack starting NextTurn.
  - The engine binds a targeted MustAttack controller at resolution (CR 508.1d).
- **collection-casts** (Hellcarver Demon, Izzet Chemister, Kylox, Doom Reigns Supreme, Kaho, Krang & Shredder, Shell of the Last Kappa, Boiling Rock Rioter, Jeleva, Summon: Esper Valigarmanda, Chandra Ablaze, Forger's Foundry)
  - New collection-cast clause primitive in permission_helpers/collection_casts.rs → ChooseObjects + ForEachTagged CastTagged (CR 608.2g, 607.2a).
- **life-bid-procedure** (Illicit Auction): the existing bundle grammar is now reachable through a five-sentence pair procedure.
- **attacked-turn-permissions** (Boros Strike-Captain, Goblin Researcher, Neriv, Neyali, Robber of the Rich)
  - New AttackedWithTurnCondition on GrantPlayTaggedEffect → GrantSource::EffectDuringTurnsAttackedWith, evaluated from turn-history CreaturesAttackedWith.
  - Changes span core, engine, interpreter, AST, lowering, grammar and renderer.
- **qualified-lure-requirements** (You Look Upon the Tarrasque): "all creatures <filter> able to block X do so" with a qualified blocker filter (CR 509.1c).
- **tapped-attacking-entry-riders** (Grim Reaper): the chain split now keeps "tapped and attacking". Speculative — the root cause is unconfirmed on current main.
- **controller-life-doubling** (Celestial Mantle): "double its controller's life total".
- **source-and-each-exile** (Ajani, Strength of the Pride; Fraying Line): "exile <source> and each <set>" is now split into two exiles.

## Blocked, by missing mechanic
The ledger's gameplay_gap field has the exact gap for each card.
- **Owned elsewhere:**
  - p05 permission variants (14 cards) and attack-toward-player (4)
  - p06 would-X-instead (8)
  - p11 repeat loops (2)
  - p09 predicates (2)
  - p10 library look/rest procedures (6)
  - p12 copy-then-cast and token/characteristic shapes (13)
- **Engine gaps:**
  - loyalty-activation overrides (4)
  - conditional attack requirements (3)
  - two-pile separation (2)
  - defending-player choices (5)
  - divided prevention (2)
  - Blight and Behold costs
  - cloak from hand
  - outside-the-game casting
  - Attractions and stickers
  - targetless life auction
  - per-player targets
  - per-type casting
  - control of a player's next turn
  - chosen attackers
  - block assignment and reassignment
  - labeled piles
  - dynamic activation limit
  - extra-turn restriction
- **Mine and tractable next:**
  - Blech, Tawnos's Tinkering, Brigid (counter/damage target-list grammar)
  - Chaos Moon, Rumbling Ruin (count-then-reference)
  - Liege of the Tangle, Minas Morgul, Ultima (for-as-long-as-counter animation)
  - Storm of Souls
  - Oskar, Sproutback Trudge, Syrix (graveyard self/trigger casts)
  - Crabomination
  - Meddle, Quicksilver Dragon
  - remaining singletons

## Risks
- **attacked-turn-permissions overlaps p05's permission area:**
  - it adds a field to GrantPlayTaggedEffect (serde-default)
  - it adds a GrantSource/GrantLifetime variant
  - it adds an AST field on GrantPlayTaggedForAsLongAsExiled
  - expect conflicts in grant_play_tagged.rs, grant_registry.rs, subject_verb_middle.rs and the text guard lines
- **The collection-cast primitive** defers to cast-or-play-tagged-clause whenever that reader returns Some. Another package adding cast readings with heads you/cast could still create registry ambiguity.
- **Unverified reference resolution:**
  - the "exiled this way" alias
  - untyped "them" pools in ChooseObjects
  - Celestial Mantle's "its"
- **Grim Reaper and Tidebinder Mage** need re-measurement on a current build.
