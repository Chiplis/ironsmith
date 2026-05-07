import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/c18/RavenousSlimeTest.java",
  "tests": [
    {
      "name": "testRavenousSlime",
      "operations": [
        {
          "op": "unsupported",
          "source": "addCustomEffect_TargetDestroy(playerA)"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Ravenous Slime",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Runeclaw Bear",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Centaur Courser",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "target destroy",
          "target": "Runeclaw Bear"
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "BEGIN_COMBAT",
          "power": 0,
          "toughness": "Ravenous Slime"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Ravenous Slime",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Centaur Courser",
          "attacker": "Ravenous Slime"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "op": "assertExileCount",
          "player": 1,
          "name": "Runeclaw Bear",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 1,
          "name": "Centaur Courser",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Ravenous Slime",
          "count": 1
        }
      ]
    }
  ]
});
