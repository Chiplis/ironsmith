import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/war/PriceOfBetrayalTest.java",
  "tests": [
    {
      "name": "testPriceOfBetrayalPermanent",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Price of Betrayal",
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "addCounters(1, PhaseStep.PRECOMBAT_MAIN, playerA, bearCub, CounterType.P1P1, 3)"
        },
        {
          "op": "unsupported",
          "source": "addCounters(1, PhaseStep.PRECOMBAT_MAIN, playerA, bearCub, CounterType.CHARGE, 1)"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Price of Betrayal",
          "target": "Bear Cub"
        },
        {
          "op": "setCounterChoice",
          "player": 0,
          "counts": {
            "CHARGE": 1,
            "P1P1": 3
          }
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Bear Cub",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Bear Cub",
          "counter": "CHARGE",
          "count": 0
        }
      ]
    },
    {
      "name": "testPriceOfBetrayalPlayer",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "add counter",
          "custom": true,
          "fixtureSource": true,
          "typeLine": "",
          "oracleText": "{0}: Target player gets five energy counters."
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Price of Betrayal",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "add counter",
          "target": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": 1,
          "counter": "ENERGY",
          "count": 5
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Price of Betrayal",
          "target": 1
        },
        {
          "op": "setCounterChoice",
          "player": 0,
          "counts": {
            "ENERGY": 5
          }
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": 1,
          "counter": "ENERGY",
          "count": 0
        }
      ]
    }
  ]
});
