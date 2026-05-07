import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/spm/TerrificTeamUpTest.java",
  "tests": [
    {
      "name": "testTerrificTeamUp",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Bear Cub",
          "count": 2
        },
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
          "player": 1,
          "name": "Sea Monster",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Terrific Team-Up",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Terrific Team-Up"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Bear Cub"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Sea Monster"
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
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Sea Monster",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Bear Cub",
          "power": 3,
          "toughness": 2
        }
      ]
    }
  ]
});
