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
          "op": "addObjectCounters",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Island",
          "counter": "ACORN",
          "count": 2
        },
        {
          "op": "addObjectCounters",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Island",
          "counter": "AEGIS",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Move a counter of each kind not on this creature from another target permanent you control onto this creature.",
          "target": "Island",
          "source": "Goldberry, River-Daughter"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertCounterCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Island",
          "counter": "ACORN",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Island",
          "counter": "AEGIS",
          "count": 0
        },
        {
          "op": "assertCounterCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Goldberry, River-Daughter",
          "counter": "ACORN",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Goldberry, River-Daughter",
          "counter": "AEGIS",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{U}, {T}: Move one or more counters from this creature onto another target permanent you control. If you do, draw a card.",
          "target": "Island",
          "source": "Goldberry, River-Daughter"
        },
        {
          "op": "assertChoiceMenu",
          "kind": "number",
          "player": 0,
          "description": "Choose how many acorn counters to move from Goldberry, River-Daughter",
          "value": 0
        },
        {
          "op": "assertChoiceMenu",
          "kind": "number",
          "player": 0,
          "description": "Choose how many aegis counters to move from Goldberry, River-Daughter",
          "value": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
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
          "counter": "ACORN",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Island",
          "counter": "AEGIS",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Goldberry, River-Daughter",
          "counter": "ACORN",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Goldberry, River-Daughter",
          "counter": "AEGIS",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 2
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
          "op": "addObjectCounters",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Island",
          "counter": "ACORN",
          "count": 2
        },
        {
          "op": "addObjectCounters",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Goldberry, River-Daughter",
          "counter": "ACORN",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Move a counter of each kind not on this creature from another target permanent you control onto this creature.",
          "target": "Island",
          "source": "Goldberry, River-Daughter"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
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
          "counter": "ACORN",
          "count": 2
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Goldberry, River-Daughter",
          "counter": "ACORN",
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
          "op": "addObjectCounters",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Goldberry, River-Daughter",
          "counter": "ACORN",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{U}, {T}: Move one or more counters from this creature onto another target permanent you control. If you do, draw a card.",
          "target": "Island",
          "source": "Goldberry, River-Daughter"
        },
        {
          "op": "assertChoiceMenu",
          "kind": "number",
          "player": 0,
          "description": "Choose how many acorn counters to move from Goldberry, River-Daughter",
          "rejectedValue": 0,
          "value": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
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
          "counter": "ACORN",
          "count": 0
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Island",
          "counter": "ACORN",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 1
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
          "ability": "{U}, {T}: Move one or more counters from this creature onto another target permanent you control. If you do, draw a card.",
          "target": "Island",
          "source": "Goldberry, River-Daughter"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
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
          "count": 0
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
          "op": "addObjectCounters",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Island",
          "counter": "M1M1",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Move a counter of each kind not on this creature from another target permanent you control onto this creature.",
          "target": "Island",
          "source": "Goldberry, River-Daughter"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
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
          "counter": "M1M1",
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
