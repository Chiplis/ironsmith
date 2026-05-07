import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/enchantments/StarfieldOfNyxTest.java",
  "tests": [
    {
      "name": "testCloudform",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Thopter Spy Network",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 8
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
          "zone": "HAND",
          "player": 0,
          "name": "Starfield of Nyx",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Cloudform",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Cloudform",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Starfield of Nyx"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Cloudform"
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
          "name": "Thopter Spy Network",
          "count": 0
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "EmptyNames.FACE_DOWN_CREATURE.getTestCommand()",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Starfield of Nyx",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Thopter Spy Network",
          "power": 4,
          "toughness": 4
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Cloudform",
          "count": 2
        }
      ]
    },
    {
      "name": "testHexproof",
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
          "zone": "HAND",
          "player": 0,
          "name": "Starfield of Nyx",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Singing Bell Strike",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Silumgar, the Drifting Death",
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
          "name": "Starfield of Nyx"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Yes"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Singing Bell Strike"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Silumgar, the Drifting Death"
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
          "name": "Starfield of Nyx",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Singing Bell Strike",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "if (enchantment != null && enchantment.getAttachedTo() != null) { Permanent enchanted = currentGame.getPermanent(enchantment.getAttachedTo()); Assert.assertEquals(\"Silumgar was enchanted\", \"Silumgar, the Drifting Death\", enchanted.getName()); } else { Assert.fail(\"Singing Bell Strike not on the battlefield\"); }"
        }
      ]
    },
    {
      "name": "testStarfieldOfNyxLayers",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Starfield of Nyx",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Humility",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Master of the Feast",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Emrakul, the Aeons Torn",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Crusade",
          "count": 4
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Master of the Feast",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Humility",
          "power": 4,
          "toughness": 4
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Emrakul, the Aeons Torn",
          "power": 15,
          "toughness": 15
        }
      ]
    },
    {
      "name": "testStarfieldOfNyxAndSongOfTheDryads",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Always Watching",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Starfield of Nyx",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Forest",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Song of the Dryads",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Starfield of Nyx"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Song of the Dryads",
          "target": "Starfield of Nyx"
        },
        {
          "op": "setStopAt",
          "turn": 2,
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
          "name": "Always Watching",
          "count": 5
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Song of the Dryads",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Always Watching\", CardType.ENCHANTMENT, true)"
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Always Watching\", CardType.CREATURE, false)"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Always Watching",
          "power": 0,
          "toughness": 0
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Starfield of Nyx\", CardType.LAND, SubType.FOREST)"
        }
      ]
    }
  ]
});
