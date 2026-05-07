import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/thb/AspectOfManticoreTest.java",
  "tests": [
    {
      "name": "test_GivesFirstStrikeTemporary",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
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
          "name": "Aspect of Manticore",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Aspect of Manticore",
          "target": "Memnite"
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "PRECOMBAT_MAIN",
          "power": 0,
          "toughness": "Memnite"
        },
        {
          "op": "assertAbility",
          "player": 1,
          "name": "PRECOMBAT_MAIN",
          "ability": 0,
          "expected": "Memnite"
        },
        {
          "op": "assertPowerToughness",
          "player": "Memnite is 3/1 turn 2",
          "name": 2,
          "power": "PRECOMBAT_MAIN",
          "toughness": 0
        },
        {
          "op": "assertAbility",
          "player": "Memnite doesn't have first strike turn 2",
          "name": 2,
          "ability": "PRECOMBAT_MAIN",
          "expected": 0
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Aspect of Manticore",
          "count": 1
        }
      ]
    }
  ]
});
