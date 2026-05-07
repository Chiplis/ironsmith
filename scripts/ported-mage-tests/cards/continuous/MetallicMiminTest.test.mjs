import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/continuous/MetallicMiminTest.java",
  "tests": [
    {
      "name": "testMetallicMimic",
      "operations": [
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
          "name": "Metallic Mimic",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Metallic Mimic"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Dwarf"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Metallic Mimic"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Dwarf"
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
          "name": "Metallic Mimic",
          "count": 2
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Metallic Mimic",
          "power": 2,
          "toughness": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Metallic Mimic",
          "power": 3,
          "toughness": 2
        }
      ]
    },
    {
      "name": "testMetallicMimicBramblewoodParagon",
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
          "name": "Metallic Mimic",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Bramblewood Paragon",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Metallic Mimic"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Warrior"
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
          "name": "Metallic Mimic",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Metallic Mimic",
          "power": 3,
          "toughness": 2
        }
      ]
    },
    {
      "name": "testMetallicLasts",
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
          "name": "Metallic Mimic",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Howlpack Resurgence",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Metallic Mimic"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Wolf"
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
          "name": "Metallic Mimic",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Metallic Mimic",
          "power": 3,
          "toughness": 2
        }
      ]
    }
  ]
});
