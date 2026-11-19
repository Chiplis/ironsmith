import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/afc/IndomitableMightTest.java",
  "tests": [
    {
      "name": "testAsThoughEffect",
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Runeclaw Bear",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Centaur Courser",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Azure Drake",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Fortress Crab",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Indomitable Might",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Indomitable Might",
          "target": "Runeclaw Bear",
          "holdStack": true
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Indomitable Might",
          "count": 1,
          "isSpell": true,
          "objectTarget": "Runeclaw Bear"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Runeclaw Bear",
          "power": 5,
          "toughness": 5
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Runeclaw Bear",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Centaur Courser",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Azure Drake",
          "attacker": "Centaur Courser"
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Fortress Crab",
          "attacker": "Runeclaw Bear"
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "distribution",
          "description": "Assign 5 combat damage from Runeclaw Bear (assign exactly 5 total)",
          "choices": [
            "Bob",
            "Bob",
            "Bob",
            "Bob",
            "Bob"
          ]
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 15
        },
        {
          "op": "assertDamageReceived",
          "player": 0,
          "name": "Runeclaw Bear",
          "damage": 1
        },
        {
          "op": "assertDamageReceived",
          "player": 0,
          "name": "Centaur Courser",
          "damage": 2
        },
        {
          "op": "assertDamageReceived",
          "player": 1,
          "name": "Azure Drake",
          "damage": 3
        },
        {
          "op": "assertDamageReceived",
          "player": 1,
          "name": "Fortress Crab",
          "damage": 0
        }
      ]
    }
  ]
});
