import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/watchers/FellShepherdTest.java",
  "tests": [
    {
      "name": "testCreaturesReturn",
      "operations": [
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
          "name": "Craw Wurm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Raging Goblin",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Goblin Roughrider",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Fell Shepherd",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{B}, Sacrifice another creature: Target creature gets -2/-2 until end of turn.",
          "target": "Raging Goblin"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Fell Shepherd",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 12
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Goblin Roughrider",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Raging Goblin",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Craw Wurm",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Craw Wurm",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Raging Goblin",
          "count": 1
        }
      ]
    }
  ]
});
