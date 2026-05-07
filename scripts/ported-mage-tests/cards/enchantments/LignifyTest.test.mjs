import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/enchantments/LignifyTest.java",
  "tests": [
    {
      "name": "LooseType",
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
          "name": "Lignify",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Sliver Hivelord",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Lignify",
          "target": "Sliver Hivelord"
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
          "name": "Lignify",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Sliver Hivelord",
          "count": 1
        },
        {
          "op": "assertAbility",
          "player": 1,
          "name": "Sliver Hivelord",
          "ability": "Indestructible",
          "expected": false
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Sliver Hivelord",
          "power": 0,
          "toughness": 4
        }
      ]
    }
  ]
});
