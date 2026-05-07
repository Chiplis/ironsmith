import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/som/ContagionEngineTest.java",
  "tests": [
    {
      "name": "testCountersArePut",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Contagion Engine",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Pincher Beetles",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Sacred Wolf",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Beloved Chaplain",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Contagion Engine"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Silvercoat Lion",
          "power": 1,
          "toughness": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Sacred Wolf",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Beloved Chaplain",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Pincher Beetles",
          "count": 1
        }
      ]
    },
    {
      "name": "testCountersDoubledByProliferate",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Contagion Engine",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Ajani Goldmane",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Wall of Frost",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Kalonian Behemoth",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Plated Slagwurm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Teysa, Envoy of Ghosts",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Contagion Engine"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{4}, {T}: Proliferate"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Wall of Frost^Kalonian Behemoth^Plated Slagwurm^Teysa, Envoy of Ghosts^Ajani Goldmane"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Wall of Frost^Kalonian Behemoth^Plated Slagwurm^Teysa, Envoy of Ghosts^Ajani Goldmane"
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
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Ajani Goldmane",
          "counter": "LOYALTY",
          "count": 6
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Kalonian Behemoth",
          "power": 6,
          "toughness": 6
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Plated Slagwurm",
          "power": 5,
          "toughness": 5
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Teysa, Envoy of Ghosts",
          "power": 1,
          "toughness": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Wall of Frost",
          "power": -3,
          "toughness": 4
        }
      ]
    }
  ]
});
