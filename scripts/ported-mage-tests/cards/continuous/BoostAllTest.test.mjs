import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/continuous/BoostAllTest.java",
  "tests": [
    {
      "name": "testBoostWithOther",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Verdeloth the Ancient",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Heartwood Treefolk",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Heartwood Treefolk",
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
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Verdeloth the Ancient",
          "power": 4,
          "toughness": 7
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Heartwood Treefolk",
          "power": 4,
          "toughness": 5
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Heartwood Treefolk",
          "power": 4,
          "toughness": 5
        }
      ]
    },
    {
      "name": "testTribalUnity",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Tribal Unity",
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
          "name": "Akki Blizzard-Herder",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Bloom Tender",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Akki Blizzard-Herder",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Bloom Tender",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Tribal Unity"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=2"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Elf"
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
          "player": 0,
          "name": "Akki Blizzard-Herder",
          "power": 1,
          "toughness": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Bloom Tender",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Akki Blizzard-Herder",
          "power": 1,
          "toughness": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Bloom Tender",
          "power": 3,
          "toughness": 3
        }
      ]
    }
  ]
});
