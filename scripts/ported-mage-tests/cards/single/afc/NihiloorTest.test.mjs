import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/afc/NihiloorTest.java",
  "tests": [
    {
      "name": "returnsControlOfPermenantThatChangedTypes",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Nihiloor",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Afterlife",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Aggressive Mammoth",
          "count": 1
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
          "name": "Island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Iroas, God of Victory",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Aerathi Berserker",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Nihiloor"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Aggressive Mammoth"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Iroas, God of Victory"
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
          "name": "Nihiloor",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Iroas, God of Victory",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Iroas, God of Victory",
          "count": 0
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Afterlife",
          "target": "Nihiloor"
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
          "name": "Nihiloor",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Nihiloor",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Iroas, God of Victory",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Iroas, God of Victory",
          "count": 1
        }
      ]
    }
  ]
});
