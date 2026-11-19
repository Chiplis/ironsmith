import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/tmp/ExcavatorTest.java",
  "tests": [
    {
      "name": "testExcavator",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Excavator",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Leyline of the Guildpact",
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
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Island"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Balduvian Bears"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertKeywordCount",
          "player": 0,
          "name": "Balduvian Bears",
          "ability": "Plainswalk",
          "count": 1
        },
        {
          "op": "assertKeywordCount",
          "player": 0,
          "name": "Balduvian Bears",
          "ability": "Islandwalk",
          "count": 1
        },
        {
          "op": "assertKeywordCount",
          "player": 0,
          "name": "Balduvian Bears",
          "ability": "Swampwalk",
          "count": 1
        },
        {
          "op": "assertKeywordCount",
          "player": 0,
          "name": "Balduvian Bears",
          "ability": "Mountainwalk",
          "count": 1
        },
        {
          "op": "assertKeywordCount",
          "player": 0,
          "name": "Balduvian Bears",
          "ability": "Forestwalk",
          "count": 1
        }
      ]
    }
  ]
});
