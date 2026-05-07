import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/activated/LinvalaKeeperOfSilenceTest.java",
  "tests": [
    {
      "name": "linvalaSilencesActivatedAbilities",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Linvala, Keeper of Silence",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Jagged-Scar Archers",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Imperious Perfect",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Forest",
          "count": 2
        },
        {
          "op": "assertPlayableAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "label": "{G}:",
          "expected": false
        },
        {
          "op": "assertPlayableAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "label": "Jagged",
          "expected": false
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Linvala, Keeper of Silence",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Jagged-Scar Archers",
          "power": 3,
          "toughness": 3
        }
      ]
    }
  ]
});
