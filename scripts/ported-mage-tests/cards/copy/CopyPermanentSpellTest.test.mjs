import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/copy/CopyPermanentSpellTest.java",
  "tests": [
    {
      "name": "testSimpleToken",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forker",
          "custom": true,
          "fixtureSource": true,
          "typeLine": "",
          "power": null,
          "toughness": null,
          "oracleText": "Whenever you cast a spell, copy it. You may choose new targets for the copy."
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
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Grizzly Bears"
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
          "name": "Grizzly Bears",
          "count": 2
        }
      ]
    },
    {
      "name": "testAuraToken",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forker",
          "custom": true,
          "fixtureSource": true,
          "typeLine": "",
          "power": null,
          "toughness": null,
          "oracleText": "Whenever you cast a spell, copy it. You may choose new targets for the copy."
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
          "zone": "HAND",
          "player": 0,
          "name": "Holy Strength",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": false
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Holy Strength",
          "target": "Grizzly Bears"
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
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Holy Strength",
          "count": 2
        }
      ]
    },
    {
      "name": "testAuraTokenRedirect",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forker",
          "custom": true,
          "fixtureSource": true,
          "typeLine": "",
          "power": null,
          "toughness": null,
          "oracleText": "Whenever you cast a spell, copy it. You may choose new targets for the copy."
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
          "player": 1,
          "name": "Centaur Courser",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Serra Angel",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Dead Weight",
          "count": 1
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
          "name": "Dead Weight",
          "target": "Centaur Courser"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 1,
          "text": "Cast Dead Weight",
          "objectTarget": "Centaur Courser"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 1,
          "source": "Forker",
          "abilityKind": "Triggered",
          "onTop": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Serra Angel"
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
          "count": 2,
          "text": "Cast Dead Weight"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 1,
          "text": "Cast Dead Weight",
          "objectTarget": "Serra Angel",
          "onTop": true
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 1,
          "text": "Cast Dead Weight",
          "objectTarget": "Centaur Courser"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertAttachedTo",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "attachment": "Dead Weight",
          "targetPlayer": 1,
          "target": "Serra Angel",
          "expected": true
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
          "name": "Dead Weight",
          "count": 2,
          "seatIndependent": true
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Centaur Courser",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Serra Angel",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Centaur Courser",
          "power": 1,
          "toughness": 1,
          "seatIndependent": true
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Serra Angel",
          "power": 2,
          "toughness": 2,
          "seatIndependent": true
        }
      ]
    },
    {
      "name": "testKickerTrigger",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forker",
          "custom": true,
          "fixtureSource": true,
          "typeLine": "",
          "power": null,
          "toughness": null,
          "oracleText": "Whenever you cast a spell, copy it. You may choose new targets for the copy."
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
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Goblin Bushwhacker",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Goblin Bushwhacker"
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
          "name": "Goblin Bushwhacker",
          "count": 2
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Grizzly Bears",
          "power": 4,
          "toughness": 2
        }
      ]
    },
    {
      "name": "testKickerReplacement",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forker",
          "custom": true,
          "fixtureSource": true,
          "typeLine": "",
          "power": null,
          "toughness": null,
          "oracleText": "Whenever you cast a spell, copy it. You may choose new targets for the copy."
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
          "player": 0,
          "name": "Aether Figment",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Aether Figment"
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
          "name": "Aether Figment",
          "count": 2
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Aether Figment",
          "power": 3,
          "toughness": 3
        }
      ]
    },
    {
      "name": "testSurgeTrigger",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forker",
          "custom": true,
          "fixtureSource": true,
          "typeLine": "",
          "power": null,
          "toughness": null,
          "oracleText": "Whenever you cast a spell, copy it. You may choose new targets for the copy."
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
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Reckless Bushwhacker",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Memnite",
          "resolve": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Reckless Bushwhacker with surge"
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
          "name": "Reckless Bushwhacker",
          "count": 2
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Memnite",
          "power": 3,
          "toughness": 1
        }
      ]
    },
    {
      "name": "testBestow",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forker",
          "custom": true,
          "fixtureSource": true,
          "typeLine": "",
          "power": null,
          "toughness": null,
          "oracleText": "Whenever you cast a spell, copy it. You may choose new targets for the copy."
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Nimbus Naiad",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": false
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Nimbus Naiad using bestow",
          "target": "Grizzly Bears"
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
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Grizzly Bears",
          "power": 6,
          "toughness": 6
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Nimbus Naiad",
          "count": 2
        }
      ]
    },
    {
      "name": "testBestowRedirect",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forker",
          "custom": true,
          "fixtureSource": true,
          "typeLine": "",
          "power": null,
          "toughness": null,
          "oracleText": "Whenever you cast a spell, copy it. You may choose new targets for the copy."
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Arbor Elf",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Nimbus Naiad",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Nimbus Naiad using bestow",
          "target": "Grizzly Bears"
        },
        {
          "op": "setStrictChooseMode",
          "value": false
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
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Grizzly Bears",
          "power": 6,
          "toughness": 6
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Grizzly Bears",
          "ability": "Flying",
          "expected": true
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Arbor Elf",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Arbor Elf",
          "power": 1,
          "toughness": 1
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Arbor Elf",
          "ability": "Flying",
          "expected": false
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Nimbus Naiad",
          "count": 2
        }
      ]
    },
    {
      "name": "testBestowFallOff",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forker",
          "custom": true,
          "fixtureSource": true,
          "typeLine": "",
          "power": null,
          "toughness": null,
          "oracleText": "Whenever you cast a spell, copy it. You may choose new targets for the copy."
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Underground Sea",
          "count": 8
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
          "zone": "HAND",
          "player": 0,
          "name": "Murder",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Nimbus Naiad",
          "count": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setOptionChoices",
          "description": "Choose new targets for the copy",
          "choices": [
            "No"
          ],
          "kind": "boolean",
          "player": 0
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Nimbus Naiad using bestow",
          "target": "Grizzly Bears",
          "holdStack": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Nimbus Naiad",
          "count": 2
        },
        {
          "op": "assertType",
          "player": 0,
          "name": "Nimbus Naiad",
          "cardType": "ENCHANTMENT",
          "extra": "true"
        },
        {
          "op": "setOptionChoices",
          "description": "Choose new targets for the copy",
          "choices": [
            "No"
          ],
          "kind": "boolean",
          "player": 0
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Murder",
          "target": "Grizzly Bears",
          "holdStack": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 0
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Nimbus Naiad"
          },
          "count": 2,
          "checks": [
            {
              "property": "cardType",
              "value": "Creature",
              "expected": true
            },
            {
              "property": "cardType",
              "value": "Enchantment",
              "expected": true
            },
            {
              "property": "subtype",
              "value": "Aura",
              "expected": false
            }
          ]
        },
        {
          "op": "assertTokenCount",
          "player": 0,
          "name": "Nimbus Naiad",
          "count": 1
        }
      ]
    },
    {
      "name": "testBestowRedirectFallOff",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forker",
          "custom": true,
          "fixtureSource": true,
          "typeLine": "",
          "power": null,
          "toughness": null,
          "oracleText": "Whenever you cast a spell, copy it. You may choose new targets for the copy."
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Underground Sea",
          "count": 8
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
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Murder",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Nimbus Naiad",
          "count": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setOptionChoices",
          "description": "Choose new targets for the copy",
          "choices": [
            "Yes"
          ],
          "kind": "boolean",
          "player": 0
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Nimbus Naiad using bestow",
          "target": "Grizzly Bears",
          "holdStack": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Silvercoat Lion"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Nimbus Naiad",
          "count": 2
        },
        {
          "op": "assertType",
          "player": 0,
          "name": "Nimbus Naiad",
          "cardType": "ENCHANTMENT",
          "extra": "true"
        },
        {
          "op": "setOptionChoices",
          "description": "Choose new targets for the copy",
          "choices": [
            "Yes"
          ],
          "kind": "boolean",
          "player": 0
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Murder",
          "target": "Silvercoat Lion",
          "holdStack": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Grizzly Bears"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 0
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Nimbus Naiad"
          },
          "count": 2,
          "checks": [
            {
              "property": "cardType",
              "value": "Creature",
              "expected": true
            },
            {
              "property": "cardType",
              "value": "Enchantment",
              "expected": true
            },
            {
              "property": "subtype",
              "value": "Aura",
              "expected": false
            }
          ]
        },
        {
          "op": "assertTokenCount",
          "player": 0,
          "name": "Nimbus Naiad",
          "count": 1
        }
      ]
    },
    {
      "name": "testBestowIllegalTarget",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forker",
          "custom": true,
          "fixtureSource": true,
          "typeLine": "",
          "power": null,
          "toughness": null,
          "oracleText": "Whenever you cast a spell, copy it. You may choose new targets for the copy."
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Underground Sea",
          "count": 8
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
          "zone": "HAND",
          "player": 0,
          "name": "Murder",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Nimbus Naiad",
          "count": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setOptionChoices",
          "description": "Choose new targets for the copy",
          "choices": [
            "No"
          ],
          "kind": "boolean",
          "player": 0
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Nimbus Naiad using bestow",
          "target": "Grizzly Bears",
          "holdStack": true
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
          "source": "Nimbus Naiad",
          "count": 2,
          "isSpell": true,
          "objectTarget": "Grizzly Bears"
        },
        {
          "op": "setOptionChoices",
          "description": "Choose new targets for the copy",
          "choices": [
            "No"
          ],
          "kind": "boolean",
          "player": 0
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Murder",
          "target": "Grizzly Bears",
          "holdStack": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 0
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Nimbus Naiad"
          },
          "count": 2,
          "checks": [
            {
              "property": "cardType",
              "value": "Creature",
              "expected": true
            },
            {
              "property": "cardType",
              "value": "Enchantment",
              "expected": true
            },
            {
              "property": "subtype",
              "value": "Aura",
              "expected": false
            }
          ]
        },
        {
          "op": "assertTokenCount",
          "player": 0,
          "name": "Nimbus Naiad",
          "count": 1
        }
      ]
    }
  ]
});
