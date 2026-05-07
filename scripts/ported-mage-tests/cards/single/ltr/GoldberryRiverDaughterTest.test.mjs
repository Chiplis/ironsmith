import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/ltr/GoldberryRiverDaughterTest.java",
  "tests": [
    {
      "name": "testHappyPath",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Goldberry, River-Daughter",
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
          "op": "unsupported",
          "source": "addCounters(1, PhaseStep.PRECOMBAT_MAIN, playerA, island, counter1, 2)"
        },
        {
          "op": "unsupported",
          "source": "addCounters(1, PhaseStep.PRECOMBAT_MAIN, playerA, island, counter2, 1)"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Move a counter of each kind not on {this} from another target permanent you control onto Goldberry.",
          "target": "Island"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1
        },
        {
          "op": "unsupported",
          "source": "checkPermanentCounters(\"First Ability\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, island, counter1, 1)"
        },
        {
          "op": "unsupported",
          "source": "checkPermanentCounters(\"First Ability\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, island, counter2, 0)"
        },
        {
          "op": "unsupported",
          "source": "checkPermanentCounters(\"First Ability\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, goldberry, counter1, 1)"
        },
        {
          "op": "unsupported",
          "source": "checkPermanentCounters(\"First Ability\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, goldberry, counter2, 1)"
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{U}, {T}: Move one or more counters from Goldberry onto another target permanent you control. If you do, draw a card.",
          "target": "Island"
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 1
        },
        {
          "op": "unsupported",
          "source": "setChoiceAmount(playerA, 0, 1)"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Island",
          "counter": "counter1",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Island",
          "counter": "counter2",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Goldberry, River-Daughter",
          "counter": "counter1",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Goldberry, River-Daughter",
          "counter": "counter2",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0,
          "name": 2
        }
      ]
    },
    {
      "name": "testCounterAlreadyOnGoldberry",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Goldberry, River-Daughter",
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
          "op": "unsupported",
          "source": "addCounters(1, PhaseStep.PRECOMBAT_MAIN, playerA, island, counter, 2)"
        },
        {
          "op": "unsupported",
          "source": "addCounters(1, PhaseStep.PRECOMBAT_MAIN, playerA, goldberry, counter, 1)"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Move a counter of each kind not on {this} from another target permanent you control onto Goldberry.",
          "target": "Island"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1
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
          "op": "assertCounterCount",
          "player": 0,
          "name": "Island",
          "counter": "counter",
          "count": 2
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Goldberry, River-Daughter",
          "counter": "counter",
          "count": 1
        }
      ]
    },
    {
      "name": "testNotMovingCounter",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Goldberry, River-Daughter",
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
          "op": "unsupported",
          "source": "addCounters(1, PhaseStep.PRECOMBAT_MAIN, playerA, goldberry, counter, 1)"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{U}, {T}: Move one or more counters from Goldberry onto another target permanent you control. If you do, draw a card.",
          "target": "Island"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1
        },
        {
          "op": "unsupported",
          "source": "setChoiceAmount(playerA, 0)"
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
          "op": "assertCounterCount",
          "player": 0,
          "name": "Goldberry, River-Daughter",
          "counter": "counter",
          "count": 0
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Island",
          "counter": "counter",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0,
          "name": 1
        }
      ]
    },
    {
      "name": "testNoCounters",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Goldberry, River-Daughter",
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
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{U}, {T}: Move one or more counters from Goldberry onto another target permanent you control. If you do, draw a card.",
          "target": "Island"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1
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
          "op": "assertHandCount",
          "player": 0,
          "count": 0,
          "name": 0
        }
      ]
    },
    {
      "name": "testM1M1Counters",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Goldberry, River-Daughter",
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
          "op": "unsupported",
          "source": "addCounters(1, PhaseStep.PRECOMBAT_MAIN, playerA, island, counter, 1)"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Move a counter of each kind not on {this} from another target permanent you control onto Goldberry.",
          "target": "Island"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1
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
          "op": "assertCounterCount",
          "player": 0,
          "name": "Goldberry, River-Daughter",
          "counter": "counter",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Goldberry, River-Daughter",
          "power": 0,
          "toughness": 2
        }
      ]
    }
  ]
});
