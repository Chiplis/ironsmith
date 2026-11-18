import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/roe/ChampionsDrakeTest.java",
  "tests": [
    {
      "name": "testCondition",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Champion's Drake",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Halimar Wavewatch",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Wastes",
          "count": 4
        },
        {
          "op": "assertCounterCount",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "name": "Halimar Wavewatch",
          "counter": "LEVEL",
          "count": 0,
          "seatIndependent": true
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "name": "Champion's Drake",
          "power": 1,
          "toughness": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Level up"
        },
        {
          "op": "assertCounterCount",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Halimar Wavewatch",
          "counter": "LEVEL",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Champion's Drake",
          "power": 1,
          "toughness": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "Level up"
        },
        {
          "op": "assertCounterCount",
          "turn": 1,
          "phase": "END_TURN",
          "player": 0,
          "name": "Halimar Wavewatch",
          "counter": "LEVEL",
          "count": 2,
          "seatIndependent": true
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "END_TURN",
          "player": 0,
          "name": "Champion's Drake",
          "power": 1,
          "toughness": 1
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Level up"
        },
        {
          "op": "assertCounterCount",
          "turn": 3,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Halimar Wavewatch",
          "counter": "LEVEL",
          "count": 3,
          "seatIndependent": true
        },
        {
          "op": "assertPowerToughness",
          "turn": 3,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Champion's Drake",
          "power": 4,
          "toughness": 4
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "Level up"
        },
        {
          "op": "assertCounterCount",
          "turn": 3,
          "phase": "END_TURN",
          "player": 0,
          "name": "Halimar Wavewatch",
          "counter": "LEVEL",
          "count": 4,
          "seatIndependent": true
        },
        {
          "op": "assertPowerToughness",
          "turn": 3,
          "phase": "END_TURN",
          "player": 0,
          "name": "Champion's Drake",
          "power": 4,
          "toughness": 4
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Halimar Wavewatch",
          "power": 0,
          "toughness": 6
        }
      ]
    }
  ]
});
