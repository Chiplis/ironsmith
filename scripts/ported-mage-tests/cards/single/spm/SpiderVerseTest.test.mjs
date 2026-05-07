import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/spm/SpiderVerseTest.java",
  "tests": [
    {
      "name": "testSpiderVerse",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Spider-Verse",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Ashiok, Nightmare Muse",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "EXILED",
          "player": 1,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "EXILED",
          "player": 1,
          "name": "Bear Cub",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "addCounters(1, PhaseStep.PRECOMBAT_MAIN, playerA, ashiokNightmareMuse, CounterType.LOYALTY, 9)"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "-7"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Lightning Bolt"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "playerD"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": false
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "activateAbility",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "-7"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "attack",
          "turn": 5,
          "player": 0,
          "attacker": "Bear Cub",
          "defender": "playerC"
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 17
        },
        {
          "op": "assertLife",
          "player": "playerD",
          "life": 17
        },
        {
          "op": "assertLife",
          "player": "playerC",
          "life": 18
        }
      ]
    }
  ]
});
