import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/control/DackFaydenTest.java",
  "tests": [
    {
      "name": "testDackFaydenEmblem",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Dack Fayden",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Ornithopter",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Gut Shot",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 10,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Gut Shot",
          "target": "Ornithopter"
        },
        {
          "op": "setStopAt",
          "turn": 10,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 18
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Dack Fayden",
          "counter": "LOYALTY",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Ornithopter",
          "count": 1
        }
      ]
    },
    {
      "name": "testDackFaydenEmblemAcrossZones",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Dack Fayden",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Ornithopter",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Gut Shot",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Unsummon",
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
          "op": "castSpell",
          "turn": 10,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Gut Shot",
          "target": "Ornithopter"
        },
        {
          "op": "castSpell",
          "turn": 10,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Unsummon",
          "target": "Ornithopter",
          "waitForStack": "WHILE_NOT_ON_STACK"
        },
        {
          "op": "castSpell",
          "turn": 10,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Ornithopter"
        },
        {
          "op": "setStopAt",
          "turn": 10,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 18
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Dack Fayden",
          "counter": "LOYALTY",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Ornithopter",
          "count": 1
        }
      ]
    }
  ]
});
