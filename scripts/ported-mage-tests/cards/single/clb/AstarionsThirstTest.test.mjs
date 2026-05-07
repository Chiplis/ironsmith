import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/clb/AstarionsThirstTest.java",
  "tests": [
    {
      "name": "canAddCounters",
      "operations": [
        {
          "op": "addCard",
          "zone": "COMMAND",
          "player": 0,
          "name": "Akiri, Line-Slinger",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Astarion's Thirst",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 4
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
          "name": "Plains",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerD",
          "name": "Ancient Bronze Dragon",
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
          "name": "Akiri, Line-Slinger"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Astarion's Thirst",
          "target": "Ancient Bronze Dragon"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Akiri, Line-Slinger",
          "power": 7,
          "toughness": 10
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Akiri, Line-Slinger",
          "counter": "P1P1",
          "count": 7
        }
      ]
    }
  ]
});
