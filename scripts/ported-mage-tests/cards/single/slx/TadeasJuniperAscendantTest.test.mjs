import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/slx/TadeasJuniperAscendantTest.java",
  "tests": [
    {
      "name": "testAttackerLessThanTadeasAttackBlockerPowerEqualAttacker",
      "operations": [
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Canopy Spider",
          "defender": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
    },
    {
      "name": "testAttackerLessThanTadeasAttackBlockerPowerMoreThanAttacker",
      "operations": [
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Sweet-Gum Recluse",
          "defender": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
    },
    {
      "name": "testAttackerLessThanTadeasAttackWithoutReachBlockerPowerMoreThanAttacker",
      "operations": [
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Canopy Spider",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Acolyte of Xathrid",
          "defender": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
    },
    {
      "name": "testAttackerEqualTadeasAttackBlockerPowerLessThanAttacker",
      "operations": [
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Canopy Spider",
          "defender": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
    },
    {
      "name": "testAttackerEqualTadeasAttackBlockerPowerEqualAttacker",
      "operations": [
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Canopy Spider",
          "defender": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
    },
    {
      "name": "testAttackerEqualTadeasAttackBlockerPowerMoreThanAttacker",
      "operations": [
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Canopy Spider",
          "defender": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
    },
    {
      "name": "testAttackerMoreThanTadeasAttackBlockerPowerLessThanAttacker",
      "operations": [
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Brood Weaver",
          "defender": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
    },
    {
      "name": "testAttackerMoreThanTadeasAttackBlockerPowerEqualAttacker",
      "operations": [
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Brood Weaver",
          "defender": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
    },
    {
      "name": "testAttackerMoreThanTadeasAttackBlockerPowerMoreThanAttacker",
      "operations": [
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Brood Weaver",
          "defender": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
