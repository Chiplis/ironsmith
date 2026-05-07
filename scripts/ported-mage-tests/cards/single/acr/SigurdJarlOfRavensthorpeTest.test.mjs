import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/acr/SigurdJarlOfRavensthorpeTest.java",
  "tests": [
    {
      "name": "testSigurd",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Sigurd, Jarl of Ravensthorpe",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Bear Cub",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Urza's Saga",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "playLand",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Urza's Saga"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Bear Cub"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "I - {this} gains"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "unsupported",
          "source": "checkPermanentCounters(\"Saga entering counter\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, saga, CounterType.LORE, 1)"
        },
        {
          "op": "unsupported",
          "source": "checkPermanentCounters(\"Bear cub single counter\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, bear, CounterType.P1P1, 1)"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Sigurd, Jarl of Ravensthorpe",
          "defender": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "Boast &mdash; {1}: ",
          "target": "Urza's Saga"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "II - {this} gains"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Bear Cub"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": null
        },
        {
          "op": "unsupported",
          "source": "checkPermanentCounters(\"Saga boast counter\", 1, PhaseStep.POSTCOMBAT_MAIN, playerA, saga, CounterType.LORE, 2)"
        },
        {
          "op": "unsupported",
          "source": "checkPermanentCounters(\"Bear cub two counters\", 1, PhaseStep.POSTCOMBAT_MAIN, playerA, bear, CounterType.P1P1, 2)"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        }
      ]
    }
  ]
});
