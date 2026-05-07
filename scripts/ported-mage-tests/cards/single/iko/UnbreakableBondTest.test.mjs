import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/iko/UnbreakableBondTest.java",
  "tests": [
    {
      "name": "testLifelinkCounter",
      "operations": [
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Barony Vampire",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Unbreakable Bond",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 5
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Unbreakable Bond",
          "target": "Barony Vampire"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "name": "Unbreakable Bond",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Barony Vampire",
          "counter": "LIFELINK",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Barony Vampire",
          "power": 3,
          "toughness": 2
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Barony Vampire",
          "ability": "Lifelink",
          "expected": true
        }
      ]
    }
  ]
});
