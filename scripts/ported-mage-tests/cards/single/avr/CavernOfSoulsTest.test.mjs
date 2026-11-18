import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/avr/CavernOfSoulsTest.java",
  "tests": [
    {
      "name": "testCastDrake",
      "operations": [
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
          "name": "Cavern of Souls",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Azure Drake",
          "count": 1
        },
        {
          "op": "playLand",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Cavern of Souls"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Drake"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Azure Drake"
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
          "name": "Azure Drake",
          "count": 1
        }
      ]
    },
    {
      "name": "testNoCastBecauseOfCreatureType",
      "operations": [
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
          "name": "Cavern of Souls",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Abuna Acolyte",
          "count": 1
        },
        {
          "op": "playLand",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Cavern of Souls"
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "options",
          "description": "Choose one",
          "choices": [
            "Human"
          ]
        },
        {
          "op": "assertCastMethods",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Abuna Acolyte",
          "methods": [],
          "zone": "hand"
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
          "name": "Abuna Acolyte",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Cavern of Souls",
          "count": 1
        }
      ]
    },
    {
      "name": "testDrakeCountered",
      "operations": [
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
          "name": "Island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Azure Drake",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Remove Soul",
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
          "op": "playLand",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Island"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Azure Drake"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Remove Soul"
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
          "name": "Azure Drake",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Azure Drake",
          "count": 1
        }
      ]
    },
    {
      "name": "testDrakeCantBeCountered",
      "operations": [
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
          "name": "Cavern of Souls",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Azure Drake",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Remove Soul",
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
          "op": "setChoice",
          "player": 0,
          "value": "Drake"
        },
        {
          "op": "playLand",
          "player": 0,
          "name": "Cavern of Souls",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Cavern of Souls",
          "count": 1
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "colors",
          "description": "Add one mana of any color",
          "choices": [
            "Blue"
          ]
        },
        {
          "op": "activateManaAbility",
          "source": "Cavern of Souls",
          "ability": "Add one mana of any color",
          "player": 0,
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertManaPool",
          "player": 0,
          "colors": "U",
          "amount": 1
        },
        {
          "op": "castSpell",
          "name": "Azure Drake",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertStackObject",
          "source": "Azure Drake",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "castSpell",
          "name": "Remove Soul",
          "player": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "target": "Azure Drake"
        },
        {
          "op": "assertStackObject",
          "source": "Remove Soul",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "objectTarget": "Azure Drake"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Remove Soul",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Azure Drake",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Azure Drake",
          "count": 1
        }
      ]
    },
    {
      "name": "testDrakeCanBeCountered",
      "operations": [
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
          "name": "Island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Cavern of Souls",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Azure Drake",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Remove Soul",
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
          "op": "setChoice",
          "player": 0,
          "value": "Drake"
        },
        {
          "op": "playLand",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Cavern of Souls"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Azure Drake"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Remove Soul"
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
          "player": 1,
          "name": "Remove Soul",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Azure Drake",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Azure Drake",
          "count": 0
        }
      ]
    },
    {
      "name": "testConditionlManaWorksIfCavernIsReplayed",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Cavern of Souls",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Gladecover Scout",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Fume Spitter",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Boomerang",
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
          "op": "setChoice",
          "player": 0,
          "value": "Elf"
        },
        {
          "op": "playLand",
          "player": 0,
          "name": "Cavern of Souls",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Cavern of Souls",
          "count": 1
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "colors",
          "description": "Add one mana of any color",
          "choices": [
            "Green"
          ]
        },
        {
          "op": "activateManaAbility",
          "source": "Cavern of Souls",
          "ability": "Add one mana of any color",
          "player": 0,
          "count": 1,
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertManaPool",
          "player": 0,
          "colors": "G",
          "amount": 1
        },
        {
          "op": "castSpell",
          "name": "Boomerang",
          "player": 1,
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "target": "Cavern of Souls"
        },
        {
          "op": "assertStackObject",
          "source": "Boomerang",
          "count": 1,
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "objectTarget": "Cavern of Souls"
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Cavern of Souls",
          "count": 1
        },
        {
          "op": "assertManaPool",
          "player": 0,
          "colors": "G",
          "amount": 1
        },
        {
          "op": "castSpell",
          "name": "Gladecover Scout",
          "player": 0,
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertStackObject",
          "source": "Gladecover Scout",
          "count": 1,
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Gladecover Scout",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Horror"
        },
        {
          "op": "playLand",
          "player": 0,
          "name": "Cavern of Souls",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Cavern of Souls",
          "count": 1
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "colors",
          "description": "Add one mana of any color",
          "choices": [
            "Black"
          ]
        },
        {
          "op": "activateManaAbility",
          "source": "Cavern of Souls",
          "ability": "Add one mana of any color",
          "player": 0,
          "count": 1,
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "assertManaPool",
          "player": 0,
          "colors": "B",
          "amount": 1
        },
        {
          "op": "castSpell",
          "name": "Fume Spitter",
          "player": 0,
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "assertStackObject",
          "source": "Fume Spitter",
          "count": 1,
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Fume Spitter",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Boomerang",
          "count": 1
        }
      ]
    },
    {
      "name": "testCastWithColorlessManaCanBeCountered",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Cavern of Souls",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Return to the Ranks",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Counterspell",
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
          "op": "playLand",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Cavern of Souls"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Drake"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Return to the Ranks"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=1"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Counterspell",
          "target": "Return to the Ranks"
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
          "name": "Return to the Ranks",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Counterspell",
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
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 0
        }
      ]
    },
    {
      "name": "testUseWithConversionInPlay",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Cavern of Souls",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Desert Drake",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Contamination",
          "count": 1
        },
        {
          "op": "playLand",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Cavern of Souls"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Drake"
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Desert",
          "expected": false
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
          "name": "Cavern of Souls",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Desert Drake",
          "count": 0
        }
      ]
    },
    {
      "name": "testBouncedCreatureNotCountered",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Cavern of Souls",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Runeclaw Bear@bear",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Counterspell",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Unsummon",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 5
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Bear"
        },
        {
          "op": "playLand",
          "player": 0,
          "name": "Cavern of Souls",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Cavern of Souls",
          "count": 1
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "colors",
          "description": "Add one mana of any color",
          "choices": [
            "Green"
          ]
        },
        {
          "op": "activateManaAbility",
          "source": "Cavern of Souls",
          "ability": "Add one mana of any color",
          "player": 0,
          "count": 1,
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertManaPool",
          "player": 0,
          "colors": "G",
          "amount": 1
        },
        {
          "op": "castSpell",
          "name": "Runeclaw Bear",
          "player": 0,
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "objectAlias": "@bear"
        },
        {
          "op": "assertStackObject",
          "source": "Runeclaw Bear",
          "count": 1,
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "castSpell",
          "name": "Counterspell",
          "player": 1,
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "target": "Runeclaw Bear"
        },
        {
          "op": "assertStackObject",
          "source": "Counterspell",
          "count": 1,
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "objectTarget": "Runeclaw Bear"
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "@bear",
          "count": 1
        },
        {
          "op": "castSpell",
          "name": "Unsummon",
          "player": 1,
          "turn": 3,
          "phase": "BEGIN_COMBAT",
          "target": "Runeclaw Bear"
        },
        {
          "op": "assertStackObject",
          "source": "Unsummon",
          "count": 1,
          "turn": 3,
          "phase": "BEGIN_COMBAT",
          "objectTarget": "Runeclaw Bear"
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "@bear",
          "count": 1
        },
        {
          "op": "playLand",
          "player": 0,
          "name": "Forest",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "castSpell",
          "name": "Runeclaw Bear",
          "player": 0,
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "objectAlias": "@bear"
        },
        {
          "op": "assertStackObject",
          "source": "Runeclaw Bear",
          "count": 1,
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "castSpell",
          "name": "Counterspell",
          "player": 1,
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "target": "Runeclaw Bear"
        },
        {
          "op": "assertStackObject",
          "source": "Counterspell",
          "count": 1,
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "objectTarget": "Runeclaw Bear"
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Cavern of Souls",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "@bear",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Counterspell",
          "count": 2
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Unsummon",
          "count": 1
        }
      ]
    }
  ]
});
