import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/dynamicvalue/SewerNemesisTest.java",
  "tests": [
    {
      "name": "test1",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Sewer Nemesis",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Raging Goblin",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 1,
          "name": "Raging Goblin",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Sewer Nemesis"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "PlayerA"
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
          "name": "Sewer Nemesis",
          "count": 0
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Sewer Nemesis",
          "power": 4,
          "toughness": 4
        }
      ]
    }
  ]
});
