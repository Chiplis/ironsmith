import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/copy/CopySpellTest.java",
  "tests": [
    {
      "name": "copyChainOfVapor",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Chain of Vapor",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 10
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 10
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Pillarfield Ox",
          "count": 10
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 10
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Chain of Vapor",
          "target": "Pillarfield Ox"
        },
        {
          "op": "setOptionChoices",
          "player": 1,
          "kind": "boolean",
          "description": "Sacrifice a land of their choice. If the player does, they may copy this spell and may choose a new target for that copy",
          "choices": [
            "Yes"
          ]
        },
        {
          "op": "setObjectChoices",
          "player": 1,
          "description": "Sacrifice exactly 1 land",
          "choices": [
            "Island"
          ]
        },
        {
          "op": "assertChoiceMenu",
          "player": 1,
          "kind": "select_options",
          "description": "Copy this spell and may choose a new target for that copy"
        },
        {
          "op": "setOptionChoices",
          "player": 1,
          "kind": "boolean",
          "description": "Copy this spell and may choose a new target for that copy",
          "choices": [
            "Yes"
          ]
        },
        {
          "op": "setOptionChoices",
          "player": 1,
          "kind": "boolean",
          "description": "Choose new targets for Chain of Vapor?",
          "choices": [
            "Yes"
          ]
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "Silvercoat Lion"
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "boolean",
          "description": "Sacrifice a land of their choice. If the player does, they may copy this spell and may choose a new target for that copy",
          "choices": [
            "Yes"
          ]
        },
        {
          "op": "setObjectChoices",
          "player": 0,
          "description": "Sacrifice exactly 1 land",
          "choices": [
            "Island"
          ]
        },
        {
          "op": "assertChoiceMenu",
          "player": 0,
          "kind": "select_options",
          "description": "Copy this spell and may choose a new target for that copy"
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "boolean",
          "description": "Copy this spell and may choose a new target for that copy",
          "choices": [
            "Yes"
          ]
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "boolean",
          "description": "Choose new targets for Chain of Vapor?",
          "choices": [
            "Yes"
          ]
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Pillarfield Ox"
        },
        {
          "op": "setOptionChoices",
          "player": 1,
          "kind": "boolean",
          "description": "Sacrifice a land of their choice. If the player does, they may copy this spell and may choose a new target for that copy",
          "choices": [
            "No"
          ]
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
          "op": "assertHandCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "name": "Pillarfield Ox",
          "count": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 9
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Pillarfield Ox",
          "count": 8
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Island",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Island",
          "count": 1
        }
      ]
    },
    {
      "name": "ZadaHedronGrinderBoost",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Angelic Blessing",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Zada, Hedron Grinder",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Pillarfield Ox",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Angelic Blessing",
          "target": "Zada, Hedron Grinder"
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
          "name": "Angelic Blessing",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Zada, Hedron Grinder",
          "power": 6,
          "toughness": 6
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Zada, Hedron Grinder",
          "ability": "Flying",
          "expected": true
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Pillarfield Ox",
          "power": 5,
          "toughness": 7
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Pillarfield Ox",
          "ability": "Flying",
          "expected": true
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Silvercoat Lion",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "assertAbility",
          "player": 1,
          "name": "Silvercoat Lion",
          "ability": "Flying",
          "expected": false
        }
      ]
    },
    {
      "name": "BonecrusherGiantChangeTargets_BoneTargetBoth",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Bonecrusher Giant",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Barkshell Blessing",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Savannah Lions",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "name": "Barkshell Blessing"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Bonecrusher Giant"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Grizzly Bears"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Savannah Lions"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "When you pay"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": false
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
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Bonecrusher Giant",
          "power": "4 + 2 * 2",
          "toughness": "3 + 2 * 2"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Grizzly Bears",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Savannah Lions",
          "power": 2,
          "toughness": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": "20 - 2 * 2"
        }
      ]
    },
    {
      "name": "BonecrusherGiantChangeTargets_BoneTargetFirst",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Bonecrusher Giant",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Barkshell Blessing",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Savannah Lions",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "name": "Barkshell Blessing",
          "target": "Bonecrusher Giant"
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "options",
          "description": "Choose optional costs for Barkshell Blessing",
          "choices": [
            "Conspire: special"
          ]
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "hybrid_payment",
          "description": "Choose how to pay pip 1 of Barkshell Blessing",
          "choices": [
            "{W} (White mana)"
          ]
        },
        {
          "op": "setObjectChoices",
          "player": 0,
          "description": "Choose two untapped creatures to conspire",
          "choices": [
            "Grizzly Bears",
            "Savannah Lions"
          ]
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "trigger_order",
          "description": "Order triggered abilities. The leftmost item becomes the top of your stack.",
          "choices": [
            "Barkshell Blessing\nWhen you cast this spell\nTarget creature gets +2/+2 until end of turn",
            "Bonecrusher Giant\nWhenever this permanent becomes the target of a spell\nWhenever this creature becomes the target of a spell, this creature deals 2 damage to that creature's controller"
          ]
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "boolean",
          "description": "Choose new targets for Barkshell Blessing?",
          "choices": [
            "Yes"
          ]
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Grizzly Bears"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "UPKEEP",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "UPKEEP",
          "source": "Barkshell Blessing",
          "count": 1,
          "objectTarget": "Bonecrusher Giant"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "UPKEEP",
          "source": "Barkshell Blessing",
          "count": 1,
          "objectTarget": "Grizzly Bears"
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
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Bonecrusher Giant",
          "power": 6,
          "toughness": 5
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Grizzly Bears",
          "power": 4,
          "toughness": 4
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Savannah Lions",
          "power": 2,
          "toughness": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 18
        }
      ]
    },
    {
      "name": "BonecrusherGiantChangeTargets_BoneTargetSecond",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Bonecrusher Giant",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Barkshell Blessing",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Savannah Lions",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "name": "Barkshell Blessing"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Grizzly Bears"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Grizzly Bears"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Savannah Lions"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Bonecrusher Giant"
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
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Bonecrusher Giant",
          "power": 6,
          "toughness": 5
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Grizzly Bears",
          "power": 4,
          "toughness": 4
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Savannah Lions",
          "power": 2,
          "toughness": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 18
        }
      ]
    },
    {
      "name": "ZadaHedronSilverfurPartisan",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silverfur Partisan",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Zada, Hedron Grinder",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Giant Growth",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 3
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Giant Growth",
          "target": "Zada, Hedron Grinder",
          "holdStack": true
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Zada, Hedron Grinder",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
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
          "name": "Giant Growth",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Silverfur Partisan",
          "power": 5,
          "toughness": 5
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Zada, Hedron Grinder",
          "power": 6,
          "toughness": 6
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Wolf Token",
          "count": 1
        }
      ]
    },
    {
      "name": "ZadaHedronGrinderBoostWithCharm",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Cryptic Command",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Zada, Hedron Grinder",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Pillarfield Ox",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Cryptic Command",
          "target": "mode=2Zada, Hedron Grinder"
        },
        {
          "op": "setModeChoice",
          "player": 0,
          "value": "2"
        },
        {
          "op": "setModeChoice",
          "player": 0,
          "value": "4"
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
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Silvercoat Lion",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Cryptic Command",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Zada, Hedron Grinder",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Pillarfield Ox",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 4
        }
      ]
    },
    {
      "name": "ZadaHedronGrinderAndSplicedSpell",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Evermind",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Into the Fray",
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
          "player": 0,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Zada, Hedron Grinder",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Pillarfield Ox",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Into the Fray",
          "target": "Zada, Hedron Grinder"
        },
        {
          "op": "setChoice",
          "player": 0,
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
          "name": "Into the Fray",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Evermind",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 3
        }
      ]
    },
    {
      "name": "testCurseOfEchoes",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Curse of Echoes",
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
          "name": "Curse of Echoes"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Lightning Bolt"
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": 0
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
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
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 17
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 17
        }
      ]
    },
    {
      "name": "testOnlyCopyFizzles",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Atraxa, Praetors' Voice",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Walking Ballista",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Flame Slash",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Dualcaster Mage",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 4
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Walking Ballista"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=1"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Walking Ballista"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Flame Slash",
          "target": "Atraxa, Praetors' Voice"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Dualcaster Mage"
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "Flame Slash"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "Walking Ballista"
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Remove a",
          "target": "Silvercoat Lion"
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Remove a",
          "target": "Silvercoat Lion"
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
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Dualcaster Mage",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Atraxa, Praetors' Voice",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Walking Ballista",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Flame Slash",
          "count": 1
        }
      ]
    },
    {
      "name": "testSevinnesReclamation",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Sevinne's Reclamation",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Island",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Flashback"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Mountain"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Island"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
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
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Island",
          "count": 1
        }
      ]
    },
    {
      "name": "test_AllowsMultipleInstancesOfGainedTriggers",
      "operations": [
        {
          "op": "clearZone",
          "player": 0,
          "zone": "hand"
        },
        {
          "op": "clearZone",
          "player": 0,
          "zone": "library"
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Imoti, Celebrant of Bounty",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Swamp",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Swamp",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Swamp",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Spark Double",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Alpha Tyrranax",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 6
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Spark Double"
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "options",
          "description": "Choose what to copy as this permanent enters",
          "choices": [
            "Enter as a copy of Imoti, Celebrant of Bounty"
          ]
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Imoti, Celebrant of Bounty",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Alpha Tyrranax"
        },
        {
          "op": "assertStackSize",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 3
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "cascade"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": "20 - 3 * 2"
        }
      ]
    },
    {
      "name": "test_CopiedSpellsMustUseIndependentCards",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Seasons Past",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Fork",
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
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {G}",
          "count": 6
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Seasons Past"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Fork",
          "target": "Seasons Past"
        },
        {
          "op": "assertStackSize",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 2
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null,
          "once": true
        },
        {
          "op": "assertStackSize",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 2
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "text": "Cast Seasons Past",
          "count": 2
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "once": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Grizzly Bears"
        },
        {
          "op": "assertStackSize",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "once": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Grizzly Bears"
        },
        {
          "op": "assertStackSize",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 0
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        }
      ]
    },
    {
      "name": "test_CopiedSpellsAndX_1",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Riku of Two Reflections",
          "count": 1
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
          "name": "Island",
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
          "zone": "HAND",
          "player": 0,
          "name": "Banefire",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Capricopian",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 2
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {R}",
          "count": 3
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Banefire",
          "target": 1
        },
        {
          "op": "assertChoiceMenu",
          "player": 0,
          "kind": "number",
          "descriptionPrefix": "Choose value for X",
          "value": 2
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": false
        },
        {
          "op": "assertStackObject",
          "source": "Banefire",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "xValue": 2,
          "targetCount": 1
        },
        {
          "op": "assertStackObject",
          "source": "Riku of Two Reflections",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "onTop": true,
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null,
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "source": "Banefire",
          "count": 2,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "xValue": 2,
          "targetCount": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 2
        },
        {
          "op": "assertLife",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "life": "20 - 2 * 2",
          "seatIndependent": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Capricopian"
        },
        {
          "op": "assertChoiceMenu",
          "player": 0,
          "kind": "number",
          "descriptionPrefix": "Choose value for X",
          "value": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Capricopian",
          "count": 1
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
        }
      ]
    },
    {
      "name": "test_CopiedSpellsETBCounters",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Capricopian",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Grenzo, Dungeon Warden",
          "count": 1
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
          "name": "Swamp",
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
          "zone": "HAND",
          "player": 0,
          "name": "Double Major",
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
          "name": "Island",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Capricopian"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=2"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Capricopian",
          "count": 1,
          "xValue": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Double Major",
          "target": "Capricopian"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Double Major",
          "count": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Capricopian",
          "count": 2,
          "xValue": 2
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Capricopian",
            "token": false
          },
          "count": 1,
          "checks": [
            {
              "property": "counter",
              "value": "P1P1",
              "expected": 2
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Capricopian",
            "token": true
          },
          "count": 1,
          "checks": [
            {
              "property": "counter",
              "value": "P1P1",
              "expected": 2
            }
          ]
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Grenzo, Dungeon Warden"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=2"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Grenzo, Dungeon Warden",
          "count": 1,
          "xValue": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Double Major",
          "target": "Grenzo, Dungeon Warden"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Double Major",
          "count": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Grenzo, Dungeon Warden",
          "count": 2,
          "xValue": 2
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Grenzo, Dungeon Warden",
            "token": false
          },
          "count": 1,
          "checks": [
            {
              "property": "counter",
              "value": "P1P1",
              "expected": 2
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Grenzo, Dungeon Warden",
            "token": true
          },
          "count": 1,
          "checks": [
            {
              "property": "counter",
              "value": "P1P1",
              "expected": 2
            }
          ]
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
        }
      ]
    },
    {
      "name": "test_SimpleCopy_Card",
      "operations": [
        {
          "op": "addCard",
          "player": 0,
          "zone": "BATTLEFIELD",
          "name": "Free Card Copy Fixture",
          "fixtureSource": true,
          "typeLine": "Artifact",
          "oracleText": "{0}: Exile target nonland card from your graveyard and copy it. You may cast the copy without paying its mana cost."
        },
        {
          "op": "addCard",
          "player": 0,
          "zone": "GRAVEYARD",
          "name": "Grizzly Bears"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Free Card Copy Fixture",
          "ability": "{0}",
          "target": "Grizzly Bears"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Free Card Copy Fixture",
          "abilityKind": "Activated",
          "objectTarget": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertStackSize",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 1
        },
        {
          "op": "assertStackCardCopy",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "original": "Grizzly Bears",
          "originalZone": "exile",
          "name": "Grizzly Bears",
          "manaCost": "{1}{G}",
          "typeLine": "Creature - Bear",
          "rulesText": []
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertEachObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Grizzly Bears",
            "token": true
          },
          "count": 1,
          "checks": [
            {
              "property": "power",
              "expected": 2
            },
            {
              "property": "toughness",
              "expected": 2
            }
          ]
        },
        {
          "op": "assertExileCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        }
      ]
    },
    {
      "name": "test_SimpleCopy_SplitCard",
      "operations": [
        {
          "op": "addCard",
          "player": 0,
          "zone": "BATTLEFIELD",
          "name": "Free Card Copy Fixture",
          "fixtureSource": true,
          "typeLine": "Artifact",
          "oracleText": "{0}: Exile target nonland card from your graveyard and copy it. You may cast the copy without paying its mana cost."
        },
        {
          "op": "addCard",
          "player": 0,
          "zone": "GRAVEYARD",
          "name": "Alive // Well"
        },
        {
          "op": "addCard",
          "player": 0,
          "zone": "BATTLEFIELD",
          "name": "Grizzly Bears"
        },
        {
          "op": "saveRuntimeCheckpoint",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Free Card Copy Fixture",
          "ability": "{0}",
          "target": "Alive"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Free Card Copy Fixture",
          "abilityKind": "Activated",
          "objectTarget": "Alive",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Alive"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertStackSize",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 1
        },
        {
          "op": "assertStackCardCopy",
          "note": "Port correction (CR 709.4, 702.102a): Oracle prints Fuse on each half (cards.json card_faces), so the exiled split card's combined rules text has each half's text and Fuse, and a copy of one half keeps that half's Fuse line. XMage's rules list omits the keyword.",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "original": "Alive",
          "originalZone": "exile",
          "name": "Alive",
          "manaCost": "{3}{G}",
          "typeLine": "Sorcery",
          "rulesText": [
            "Create a 3/3 green Centaur creature token.",
            "Fuse"
          ],
          "originalText": [
            "Create a 3/3 green Centaur creature token.",
            "Fuse",
            "You gain 2 life for each creature you control.",
            "Fuse"
          ]
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertEachObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Centaur Token",
            "token": true
          },
          "count": 1,
          "checks": [
            {
              "property": "power",
              "expected": 3
            },
            {
              "property": "toughness",
              "expected": 3
            }
          ]
        },
        {
          "op": "assertLife",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertExileCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Alive",
          "count": 1
        },
        {
          "op": "restoreRuntimeCheckpoint"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Free Card Copy Fixture",
          "ability": "{0}",
          "target": "Alive"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Free Card Copy Fixture",
          "abilityKind": "Activated",
          "objectTarget": "Alive",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Well"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertStackSize",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 1
        },
        {
          "op": "assertStackCardCopy",
          "note": "Port correction (CR 709.4, 702.102a): Oracle prints Fuse on each half (cards.json card_faces), so the exiled split card's combined rules text has each half's text and Fuse, and a copy of one half keeps that half's Fuse line. XMage's rules list omits the keyword.",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "original": "Alive",
          "originalZone": "exile",
          "name": "Well",
          "manaCost": "{W}",
          "typeLine": "Sorcery",
          "rulesText": [
            "You gain 2 life for each creature you control.",
            "Fuse"
          ],
          "originalText": [
            "Create a 3/3 green Centaur creature token.",
            "Fuse",
            "You gain 2 life for each creature you control.",
            "Fuse"
          ]
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertLife",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "life": 22
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Centaur Token",
          "count": 0
        },
        {
          "op": "assertExileCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Alive",
          "count": 1
        }
      ]
    },
    {
      "name": "test_SimpleCopy_AdventureCard",
      "operations": [
        {
          "op": "addCard",
          "player": 0,
          "zone": "BATTLEFIELD",
          "name": "Free Card Copy Fixture",
          "fixtureSource": true,
          "typeLine": "Artifact",
          "oracleText": "{0}: Exile target nonland card from your graveyard and copy it. You may cast the copy without paying its mana cost."
        },
        {
          "op": "addCard",
          "player": 0,
          "zone": "GRAVEYARD",
          "name": "Animating Faerie"
        },
        {
          "op": "addCard",
          "player": 0,
          "zone": "BATTLEFIELD",
          "name": "Sol Ring"
        },
        {
          "op": "saveRuntimeCheckpoint",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Free Card Copy Fixture",
          "ability": "{0}",
          "target": "Animating Faerie"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Free Card Copy Fixture",
          "abilityKind": "Activated",
          "objectTarget": "Animating Faerie",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Animating Faerie"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertStackSize",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 1
        },
        {
          "op": "assertStackCardCopy",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "original": "Animating Faerie",
          "originalZone": "exile",
          "name": "Animating Faerie",
          "manaCost": "{2}{U}",
          "typeLine": "Creature - Faerie",
          "rulesText": [
            "Flying"
          ]
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertEachObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Animating Faerie",
            "token": true
          },
          "count": 1,
          "checks": [
            {
              "property": "power",
              "expected": 2
            },
            {
              "property": "toughness",
              "expected": 2
            },
            {
              "property": "ability",
              "value": "Flying",
              "kind": "static",
              "expected": true
            }
          ]
        },
        {
          "op": "assertExileCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Animating Faerie",
          "count": 1
        },
        {
          "op": "restoreRuntimeCheckpoint"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Free Card Copy Fixture",
          "ability": "{0}",
          "target": "Animating Faerie"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Free Card Copy Fixture",
          "abilityKind": "Activated",
          "objectTarget": "Animating Faerie",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Bring to Life"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Sol Ring"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertStackSize",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 1
        },
        {
          "op": "assertStackCardCopy",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "original": "Animating Faerie",
          "originalZone": "exile",
          "name": "Bring to Life",
          "manaCost": "{2}{U}",
          "typeLine": "Sorcery - Adventure",
          "rulesText": [
            "Target noncreature artifact you control becomes a 0/0 artifact creature. Put four +1/+1 counters on it."
          ],
          "originalText": [
            "Flying"
          ]
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertCounterCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Sol Ring",
          "counter": "P1P1",
          "count": 4
        },
        {
          "op": "assertPermanentCharacteristics",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Sol Ring",
          "expected": {
            "basePower": 0,
            "baseToughness": 0,
            "power": 4,
            "toughness": 4
          }
        },
        {
          "op": "assertExileCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Animating Faerie",
          "count": 1
        }
      ]
    },
    {
      "name": "test_SimpleCopy_MDFC",
      "operations": [
        {
          "op": "addCard",
          "player": 0,
          "zone": "BATTLEFIELD",
          "name": "Free Card Copy Fixture",
          "fixtureSource": true,
          "typeLine": "Artifact",
          "oracleText": "{0}: Exile target nonland card from your graveyard and copy it. You may cast the copy without paying its mana cost."
        },
        {
          "op": "addCard",
          "player": 0,
          "zone": "BATTLEFIELD",
          "name": "Land or Spell Copy Fixture",
          "fixtureSource": true,
          "typeLine": "Artifact",
          "oracleText": "{0}: Exile target card from your graveyard and copy it. You may play the copy."
        },
        {
          "op": "addCard",
          "player": 0,
          "zone": "GRAVEYARD",
          "name": "Agadeem's Awakening"
        },
        {
          "op": "addCard",
          "player": 0,
          "zone": "GRAVEYARD",
          "name": "Memnite"
        },
        {
          "op": "saveRuntimeCheckpoint",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Free Card Copy Fixture",
          "ability": "{0}",
          "target": "Agadeem's Awakening"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Free Card Copy Fixture",
          "abilityKind": "Activated",
          "objectTarget": "Agadeem's Awakening",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Memnite"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertStackSize",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 1
        },
        {
          "op": "assertStackCardCopy",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "original": "Agadeem's Awakening",
          "originalZone": "exile",
          "name": "Agadeem's Awakening",
          "manaCost": "{X}{B}{B}{B}",
          "typeLine": "Sorcery",
          "rulesText": [
            "Return from your graveyard to the battlefield any number of target creature cards that each have a different mana value X or less."
          ]
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Agadeem's Awakening",
          "count": 1
        },
        {
          "op": "restoreRuntimeCheckpoint"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Land or Spell Copy Fixture",
          "ability": "{0}",
          "target": "Agadeem's Awakening"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Land or Spell Copy Fixture",
          "abilityKind": "Activated",
          "objectTarget": "Agadeem's Awakening",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Agadeem, the Undercrypt"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertExileCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Agadeem's Awakening",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Agadeem, the Undercrypt",
          "count": 1
        }
      ]
    },
    {
      "name": "copySpellWithKicker",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Thieving Skydiver",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Double Major",
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
          "player": 0,
          "name": "Forest",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Sol Ring",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Expedition Map",
          "count": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Forest",
          "ability": "{T}: Add {G}",
          "count": 2
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Island",
          "ability": "{T}: Add {U}",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Thieving Skydiver"
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "options",
          "description": "Choose optional costs for Thieving Skydiver",
          "choices": [
            "Kicker: 0"
          ]
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=1"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Thieving Skydiver",
          "count": 1,
          "xValue": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Double Major",
          "target": "Thieving Skydiver"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Double Major",
          "count": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Thieving Skydiver",
          "count": 2,
          "xValue": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Sol Ring"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Expedition Map"
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
          "name": "Sol Ring",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Expedition Map",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Sol Ring",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Expedition Map",
          "count": 0
        }
      ]
    },
    {
      "name": "test_CopyNeverwinterHydra",
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
          "name": "Magus Lucea Kane",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Neverwinter Hydra",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Double Major",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "<i>Psychic",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Neverwinter Hydra"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=2"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Neverwinter Hydra",
          "count": 1,
          "xValue": 2
        },
        {
          "op": "unsupported",
          "source": "setDieRollResult(playerA, 3)"
        },
        {
          "op": "unsupported",
          "source": "setDieRollResult(playerA, 4)"
        },
        {
          "op": "unsupported",
          "source": "setDieRollResult(playerA, 6)"
        },
        {
          "op": "unsupported",
          "source": "setDieRollResult(playerA, 6)"
        },
        {
          "op": "unsupported",
          "source": "setDieRollResult(playerA, 1)"
        },
        {
          "op": "unsupported",
          "source": "setDieRollResult(playerA, 1)"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Double Major",
          "target": "Neverwinter Hydra"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Double Major",
          "count": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Neverwinter Hydra",
          "count": 2,
          "xValue": 2
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 2
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Neverwinter Hydra",
          "count": 2,
          "xValue": 2
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Magus Lucea Kane"
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
          "name": "Neverwinter Hydra",
          "count": 3
        }
      ]
    },
    {
      "name": "test_CopyHydradoodle",
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
          "zone": "HAND",
          "player": 0,
          "name": "Hydradoodle",
          "count": 1,
          "fixtureSource": true,
          "typeLine": "Creature — Hydra Dog",
          "oracleText": "As this creature enters, roll X six-sided dice. This creature enters with a number of +1/+1 counters on it equal to the total of those results.\nReach\nTrample",
          "manaCost": "{X}{X}{G}{G}",
          "power": 0,
          "toughness": 0
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Double Major",
          "count": 1
        },
        {
          "op": "assertChoiceMenu",
          "kind": "number",
          "description": "Choose value for X",
          "player": 0,
          "value": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Hydradoodle"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "count": 1,
          "source": "Hydradoodle",
          "xValue": 1,
          "onTop": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Double Major",
          "target": "Hydradoodle"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "count": 1,
          "source": "Double Major",
          "objectTarget": "Hydradoodle",
          "onTop": true
        },
        {
          "op": "unsupported",
          "source": "setDieRollResult(playerA, 5)"
        },
        {
          "op": "unsupported",
          "source": "setDieRollResult(playerA, 1)"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "count": 2,
          "source": "Hydradoodle",
          "xValue": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Hydradoodle",
          "count": 2
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Hydradoodle",
            "token": true
          },
          "count": 1,
          "checks": [
            {
              "property": "counter",
              "value": "P1P1",
              "expected": 5
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Hydradoodle",
            "token": false
          },
          "count": 1,
          "checks": [
            {
              "property": "counter",
              "value": "P1P1",
              "expected": 1
            }
          ]
        }
      ]
    },
    {
      "name": "test_CopyApocalypseHydra",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Taiga",
          "count": 10
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Magus Lucea Kane",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Apocalypse Hydra",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "<i>Psychic",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Apocalypse Hydra"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=5"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Magus Lucea Kane"
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
          "name": "Apocalypse Hydra",
          "count": 2
        }
      ]
    },
    {
      "name": "test_LKI",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Swan Song",
          "count": 1
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
          "player": 0,
          "name": "Bonus Round",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 3
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
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Augmenting Automaton",
          "count": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "seatIndependent": true,
          "name": "Bird Token",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "seatIndependent": true,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "seatIndependent": true,
          "name": "Augmenting Automaton",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Mountain",
          "ability": "{T}: Add {R}",
          "count": 3
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Bonus Round"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Bolt",
          "target": "Grizzly Bears"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "text": "Cast Lightning Bolt",
          "count": 1,
          "objectTarget": "Grizzly Bears"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Augmenting Automaton"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "text": "Cast Lightning Bolt",
          "count": 2
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "text": "Cast Lightning Bolt",
          "count": 1,
          "objectTarget": "Augmenting Automaton"
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Island",
          "ability": "{T}: Add {U}",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Swan Song",
          "target": "Lightning Bolt[only copy]"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "text": "Cast Swan Song",
          "count": 1,
          "objectTarget": "Lightning Bolt",
          "objectTargetKind": "spell copy"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": false
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "text": "Cast Swan Song",
          "count": 2,
          "objectTarget": "Lightning Bolt",
          "objectTargetKind": "spell copy"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "seatIndependent": true,
          "name": "Bird Token",
          "count": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "text": "Cast Lightning Bolt",
          "count": 1,
          "objectTarget": "Grizzly Bears"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 2
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "seatIndependent": true,
          "name": "Bird Token",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "seatIndependent": true,
          "name": "Grizzly Bears",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "seatIndependent": true,
          "name": "Augmenting Automaton",
          "count": 1
        }
      ]
    },
    {
      "name": "testAbilityCantBeCopied",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Life Fixture",
          "custom": true,
          "fixtureSource": true,
          "typeLine": "Artifact",
          "oracleText": "{T}: You gain 1 life. This ability can't be copied."
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Wastes",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Lithoform Engine",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "You gain 1 life"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Copy target activated or triggered ability",
          "target": "Life Fixture"
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
          "op": "assertTappedCount",
          "name": "Lithoform Engine",
          "tapped": true,
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 21
        }
      ]
    },
    {
      "name": "testCopyOnStack",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silverquill, the Disputant",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Transpose",
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Mountain",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Transpose"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Balduvian Bears"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Transpose",
          "count": 1,
          "onTop": true,
          "abilityKind": "Triggered",
          "text": "Casualty"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Transpose",
          "count": 2,
          "onTop": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Mountain"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Mountain"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
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
          "name": "Wizard Token",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Mountain",
          "count": 2
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Transpose",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 18
        }
      ]
    }
  ]
});
