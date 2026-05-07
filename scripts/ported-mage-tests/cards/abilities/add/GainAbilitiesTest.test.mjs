import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/add/GainAbilitiesTest.java",
  "tests": [
    {
      "name": "behindTheScenesShouldOnlyGrantSkulkToCreaturesYouControl",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Behind the Scenes",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Hill Giant",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Bronze Sable",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Grizzly Bears",
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
          "op": "assertAbility",
          "player": 0,
          "name": "Hill Giant",
          "ability": "new SkulkAbility()",
          "expected": true
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Bronze Sable",
          "ability": "new SkulkAbility()",
          "expected": true
        },
        {
          "op": "assertAbility",
          "player": 1,
          "name": "Memnite",
          "ability": "new SkulkAbility()",
          "expected": false
        },
        {
          "op": "assertAbility",
          "player": 1,
          "name": "Grizzly Bears",
          "ability": "new SkulkAbility()",
          "expected": false
        }
      ]
    }
  ]
});
