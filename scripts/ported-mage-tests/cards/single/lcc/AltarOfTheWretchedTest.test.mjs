import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/lcc/AltarOfTheWretchedTest.java",
  "tests": [
    {
      "name": "testAltarOfTheWretched",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Altar of the Wretched",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Angel of Invention",
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
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Craft with one or more"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Angel of Invention"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "name": "Wretched Bonemass",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Angel of Invention",
          "count": 1
        },
        {
          "op": "assertAbilities",
          "player": 0,
          "name": "Wretched Bonemass",
          "abilities": [
            "abilities"
          ]
        }
      ]
    }
  ]
});
