import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/keywords/TransformTest.java",
  "tests": [
    {
      "name": "NissaVastwoodSeerTest",
      "operations": [
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Nissa, Vastwood Seer",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Forest",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Rootrunner",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Nissa, Vastwood Seer"
        },
        {
          "op": "playLand",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Forest"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "ability": "{G}{G}",
          "target": "Swamp"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "+1: Reveal"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Rootrunner",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Nissa, Vastwood Seer",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Nissa, Sage Animist",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Nissa, Sage Animist",
          "counter": "LOYALTY",
          "count": 4
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Forest",
          "count": 6
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Swamp",
          "count": 1
        }
      ]
    },
    {
      "name": "LilianaHereticalHealer",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Liliana, Heretical Healer",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Liliana, Heretical Healer"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 1,
          "name": "Lightning Bolt",
          "target": "Silvercoat Lion"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Liliana, Heretical Healer",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Liliana, Defiant Necromancer",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Liliana, Defiant Necromancer",
          "counter": "LOYALTY",
          "count": 3
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Zombie Token",
          "count": 1
        }
      ]
    },
    {
      "name": "LilianaHereticalHealer2",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Liliana, Heretical Healer",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Languish",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Swamp",
          "count": 4
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Languish"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Languish",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Liliana, Defiant Necromancer",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Zombie Token",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Liliana, Heretical Healer",
          "count": 1
        }
      ]
    },
    {
      "name": "TestEnchantmentToCreature",
      "operations": [
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Fireball",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Infernal Scarring",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Autumnal Gloom",
          "count": 1
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Autumnal Gloom",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Ancient of the Equinox",
          "count": 1
        }
      ]
    },
    {
      "name": "testCultOfTheWaxingMoon",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Cult of the Waxing Moon",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Hinterland Logger",
          "count": 1
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "DRAW"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Cult of the Waxing Moon",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Timber Shredder",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Wolf Token",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertAbilityCount(playerA, \"Timber Shredder\", WerewolfFrontTriggeredAbility.class, 0)"
        }
      ]
    },
    {
      "name": "testStartledAwake",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Startled Awake",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 9
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Startled Awake",
          "target": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{3}{U}{U}"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 1,
          "name": 13
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Startled Awake",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Persistent Nightmare",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Persistent Nightmare\", CardType.CREATURE, true)"
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Persistent Nightmare\", CardType.SORCERY, false)"
        }
      ]
    },
    {
      "name": "testStartledAwakeMoonmist",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Startled Awake",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Moonmist",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Tropical Island",
          "count": 11
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Maskwood Nexus",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Startled Awake",
          "target": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{3}{U}{U}"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Moonmist"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 1,
          "name": 13
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Startled Awake",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Persistent Nightmare",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Persistent Nightmare\", CardType.CREATURE, true)"
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Persistent Nightmare\", CardType.SORCERY, false)"
        }
      ]
    },
    {
      "name": "testTransformCopy",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lambholt Pacifist",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Clone",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Lambholt Pacifist"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Clone"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Lambholt Pacifist"
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Lambholt Butcher",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Lambholt Pacifist",
          "count": 1
        }
      ]
    },
    {
      "name": "testDisturbExile",
      "operations": [
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Baithook Angler",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Hook-Haunt Drifter using Disturb"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Lightning Bolt",
          "target": "Hook-Haunt Drifter"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Hook-Haunt Drifter",
          "count": 0
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Baithook Angler",
          "count": 1
        }
      ]
    },
    {
      "name": "testTransformCopyTransformed",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Uninvited Geist",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Mirror Mockery",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Uninvited Geist",
          "defender": 1
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Mirror Mockery",
          "target": "Unimpeded Trespasser"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Unimpeded Trespasser",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "COMBAT_DAMAGE"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 15
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Mirror Mockery",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Unimpeded Trespasser",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Unimpeded Trespasser",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Unimpeded Trespasser",
          "power": 3,
          "toughness": 3
        }
      ]
    },
    {
      "name": "testTransformArchangelAvacyn",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Archangel Avacyn",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Eldrazi Displacer",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Wastes",
          "count": 3
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Bolt",
          "target": "Silvercoat Lion"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null,
          "once": true
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "ability": "{2}{C}",
          "target": "Archangel Avacyn"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Eldrazi Displacer",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Avacyn, the Purifier",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Archangel Avacyn",
          "count": 1
        }
      ]
    },
    {
      "name": "testNoSpellsCastLastTurnTransformDoesNotTriggerTurn1",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Hinterland Logger",
          "count": 1
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Hinterland Logger",
          "count": 1
        }
      ]
    },
    {
      "name": "testHuntmaster",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Huntmaster of the Fells",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Eldrazi Displacer",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Plains",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Wastes",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Huntmaster of the Fells"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Eldrazi Displacer"
        },
        {
          "op": "activateAbility",
          "turn": 4,
          "phase": "UPKEEP",
          "player": 1,
          "ability": "{2}{C}",
          "target": "Huntmaster of the Fells"
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 24
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Wolf Token",
          "count": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Eldrazi Displacer",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Ravager of the Fells",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Huntmaster of the Fells",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Huntmaster of the Fells",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "assertTappedCount",
          "name": "Plains",
          "tapped": true,
          "count": 2
        },
        {
          "op": "assertTappedCount",
          "name": "Wastes",
          "tapped": true,
          "count": 1
        }
      ]
    },
    {
      "name": "testHuntmasterTransformed",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Huntmaster of the Fells",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 3
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Huntmaster of the Fells"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Silvercoat Lion"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Silvercoat Lion"
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 24
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 18
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Wolf Token",
          "count": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Ravager of the Fells",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Huntmaster of the Fells",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Huntmaster of the Fells",
          "power": 2,
          "toughness": 2
        }
      ]
    },
    {
      "name": "testWildsongHowlerTrigger",
      "operations": [
        {
          "op": "clearZone",
          "player": 0,
          "zone": "library"
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Howlpack Piper",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 50
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 4
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Howlpack Piper"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Silvercoat Lion"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Howlpack Piper"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Silvercoat Lion"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Wildsong Howler",
          "count": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Howlpack Piper",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 3
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0,
          "name": 3
        }
      ]
    },
    {
      "name": "testCopyTransformedThingInTheIce",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Thing in the Ice",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Banners Raised",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Phantasmal Image",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Thing in the Ice"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Banners Raised"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Banners Raised"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Banners Raised"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Banners Raised"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Phantasmal Image"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Banners Raised",
          "count": 4
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Thing in the Ice",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Awoken Horror",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Awoken Horror",
          "power": 7,
          "toughness": 8
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Awoken Horror",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Awoken Horror",
          "power": 7,
          "toughness": 8
        }
      ]
    },
    {
      "name": "testMoonmistDelver",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Delver of Secrets",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Moonmist",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Delver of Secrets"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Moonmist"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Delver of Secrets",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Insectile Aberration",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Moonmist"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Delver of Secrets",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Insectile Aberration",
          "count": 0
        }
      ]
    },
    {
      "name": "testMoonmistHuntmasterDressdown",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Tropical Island",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Huntmaster of the Fells",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Maskwood Nexus",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Dress Down",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Moonmist",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "name": "Dress Down"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Moonmist"
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Ravager of the Fells",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Moonmist"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Dress Down",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Huntmaster of the Fells",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "count": 0,
          "name": 8
        }
      ]
    },
    {
      "name": "testTokenCopyTransformed",
      "operations": [
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Baithook Angler",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Breeding Pool",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Croaking Counterpart",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Cast Hook-Haunt Drifter using Disturb"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Croaking Counterpart",
          "target": "Hook-Haunt Drifter"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "unsupported",
          "source": "for (Permanent token : currentGame.getBattlefield().getActivePermanents(playerA.getId(), currentGame)) { if (!(token instanceof PermanentToken)) { continue; } assertTrue(token.getSubtype(currentGame).contains(SubType.FROG)); assertEquals(ObjectColor.GREEN, token.getColor(currentGame)); }"
        }
      ]
    },
    {
      "name": "testFrontSaga",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Azusa's Many Journeys",
          "count": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Azusa's Many Journeys",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Likeness of the Seeker",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Likeness of the Seeker",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "unsupported",
          "source": "assertAbilityCount(playerA, likenessOfTheSeeker, SagaAbility.class, 0)"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 23
        }
      ]
    },
    {
      "name": "testDiesTrigger",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Baithook Angler",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Abnormal Endurance",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 1,
          "name": "Lightning Bolt",
          "target": "Baithook Angler"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Abnormal Endurance",
          "target": "Baithook Angler"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Baithook Angler",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Baithook Angler",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertTrue(getPermanent(baithookAngler, playerA).isTapped())"
        }
      ]
    },
    {
      "name": "testCloneCantReturnTransformed",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Azusa's Many Journeys",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Clever Impersonator@impersonator",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 5
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Clever Impersonator"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Azusa's Many Journeys"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "unsupported",
          "source": "addCounters(1, PhaseStep.PRECOMBAT_MAIN, playerA, \"@impersonator\", CounterType.LORE, 2)"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "II"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Azusa's Many Journeys",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Likeness of the Seeker",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Clever Impersonator",
          "count": 0
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Clever Impersonator",
          "count": 1
        }
      ]
    }
  ]
});
