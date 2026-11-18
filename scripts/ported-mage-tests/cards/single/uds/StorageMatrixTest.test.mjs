import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/uds/StorageMatrixTest.java",
  "tests": [
    {
      "name": "testOnlyAffectsActivePlayer",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Storage Matrix",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Wastes",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Unwinding Clock",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Marble Chalice",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mine Worker",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Storage Matrix"
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: You gain 1 life",
          "source": "Marble Chalice"
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "BEGIN_COMBAT",
          "player": 1,
          "ability": "{T}: You gain 1 life",
          "source": "Mine Worker"
        },
        {
          "op": "assertLife",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "life": "21",
          "seatIndependent": true
        },
        {
          "op": "assertLife",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "life": "21",
          "seatIndependent": true
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Land"
        },
        {
          "op": "assertTappedCount",
          "turn": 4,
          "phase": "UPKEEP",
          "player": 1,
          "name": "Mine Worker",
          "tapped": true,
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertTappedCount",
          "turn": 4,
          "phase": "UPKEEP",
          "player": 0,
          "name": "Marble Chalice",
          "tapped": false,
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "activateAbility",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: You gain 1 life",
          "source": "Marble Chalice"
        },
        {
          "op": "assertLife",
          "turn": 4,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "life": "22",
          "seatIndependent": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Creature"
        },
        {
          "op": "assertTappedCount",
          "turn": 5,
          "phase": "UPKEEP",
          "player": 0,
          "name": "Wastes",
          "tapped": true,
          "count": 3,
          "seatIndependent": true
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Artifact"
        },
        {
          "op": "assertTappedCount",
          "turn": 6,
          "phase": "UPKEEP",
          "player": 1,
          "name": "Mine Worker",
          "tapped": false,
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertTappedCount",
          "turn": 6,
          "phase": "UPKEEP",
          "player": 0,
          "name": "Marble Chalice",
          "tapped": false,
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "activateAbility",
          "turn": 6,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: You gain 1 life",
          "source": "Marble Chalice"
        },
        {
          "op": "setStopAt",
          "turn": 6,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 23
        }
      ]
    }
  ]
});
