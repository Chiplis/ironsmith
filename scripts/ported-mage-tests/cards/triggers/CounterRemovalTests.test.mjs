import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/triggers/CounterRemovalTests.java",
  "tests": [
    {
      "name": "CounterRemovalTest",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Basri's Solidarity",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Aether Snap",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "multicountertrigdcard",
          "fixtureSource": true,
          "typeLine": "Creature",
          "power": 1,
          "toughness": 1,
          "oracleText": "Whenever one or more counters are removed from this permanent, you lose 1 life."
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "singlecountertrigdcard",
          "fixtureSource": true,
          "typeLine": "Creature",
          "power": 1,
          "toughness": 1,
          "oracleText": "Whenever a counter is removed from this permanent, you get {E}."
        },
        {
          "op": "addXmageFixture",
          "player": 0,
          "name": "multicounterreplcard",
          "ability": "removeOneFewerCounter"
        },
        {
          "op": "addXmageFixture",
          "player": 0,
          "name": "singlecounterreplcard",
          "ability": "retainTwoCounters"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Basri's Solidarity"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Basri's Solidarity"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Basri's Solidarity"
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
          "name": "multicountertrigdcard",
          "counter": "P1P1",
          "count": 3
        },
        {
          "op": "assertCounterCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "singlecountertrigdcard",
          "counter": "P1P1",
          "count": 3
        },
        {
          "op": "assertCounterCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "multicounterreplcard",
          "counter": "P1P1",
          "count": 3
        },
        {
          "op": "assertCounterCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "singlecounterreplcard",
          "counter": "P1P1",
          "count": 3
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Aether Snap"
        },
        {
          "op": "setCounterChoice",
          "mandatory": true,
          "player": 0,
          "counts": {
            "P1P1": 3
          }
        },
        {
          "op": "setCounterChoice",
          "mandatory": true,
          "player": 0,
          "counts": {
            "P1P1": 3
          }
        },
        {
          "op": "setCounterChoice",
          "mandatory": true,
          "player": 0,
          "counts": {
            "P1P1": 3
          }
        },
        {
          "op": "setCounterChoice",
          "mandatory": true,
          "player": 0,
          "counts": {
            "P1P1": 3
          }
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "singlecountertrigdcard"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "singlecountertrigdcard"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "singlecountertrigdcard"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "text": "multicountertrigdcard",
          "count": 1,
          "onTop": true
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "text": "singlecountertrigdcard",
          "count": 3
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 4
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": "currentGame.getStartingLife() - 1"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": 0,
          "counter": "ENERGY",
          "count": 3
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "multicounterreplcard",
          "counter": "P1P1",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "singlecounterreplcard",
          "counter": "P1P1",
          "count": 2
        }
      ],
      "startingLife": 20
    }
  ],
  "turnOrder": [
    0,
    3,
    2,
    1
  ]
});
