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
          "op": "assertBlockerLegality",
          "turn": 1,
          "player": 1,
          "attacker": "Canopy Spider",
          "blocker": "Dryad Arbor",
          "expected": true
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
          "op": "assertBlockerLegality",
          "turn": 1,
          "player": 1,
          "attacker": "Sweet-Gum Recluse",
          "blocker": "Southern Elephant",
          "expected": false
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
          "op": "assertBlockerLegality",
          "turn": 1,
          "player": 1,
          "attacker": "Acolyte of Xathrid",
          "blocker": "Runeclaw Bear",
          "expected": true
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
          "op": "assertBlockerLegality",
          "turn": 1,
          "player": 1,
          "attacker": "Canopy Spider",
          "blocker": "Phyrexian Walker",
          "expected": true
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
          "op": "assertBlockerLegality",
          "turn": 1,
          "player": 1,
          "attacker": "Canopy Spider",
          "blocker": "Dryad Arbor",
          "expected": true
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
          "op": "assertBlockerLegality",
          "turn": 1,
          "player": 1,
          "attacker": "Canopy Spider",
          "blocker": "Southern Elephant",
          "expected": false
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
          "op": "assertBlockerLegality",
          "turn": 1,
          "player": 1,
          "attacker": "Brood Weaver",
          "blocker": "Dryad Arbor",
          "expected": true
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
          "op": "assertBlockerLegality",
          "turn": 1,
          "player": 1,
          "attacker": "Brood Weaver",
          "blocker": "Runeclaw Bear",
          "expected": true
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
          "op": "assertBlockerLegality",
          "turn": 1,
          "player": 1,
          "attacker": "Brood Weaver",
          "blocker": "Southern Elephant",
          "expected": false
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
  ],
  "setupOperations": [
    {
      "op": "addCard",
      "zone": "BATTLEFIELD",
      "player": 0,
      "name": "Tadeas, Juniper Ascendant",
      "count": 1
    },
    {
      "op": "addCard",
      "zone": "BATTLEFIELD",
      "player": 0,
      "name": "Sweet-Gum Recluse",
      "count": 1
    },
    {
      "op": "addCard",
      "zone": "BATTLEFIELD",
      "player": 0,
      "name": "Canopy Spider",
      "count": 1
    },
    {
      "op": "addCard",
      "zone": "BATTLEFIELD",
      "player": 0,
      "name": "Brood Weaver",
      "count": 1
    },
    {
      "op": "addCard",
      "zone": "BATTLEFIELD",
      "player": 0,
      "name": "Acolyte of Xathrid",
      "count": 1
    },
    {
      "op": "addCard",
      "zone": "BATTLEFIELD",
      "player": 1,
      "name": "Phyrexian Walker",
      "count": 1
    },
    {
      "op": "addCard",
      "zone": "BATTLEFIELD",
      "player": 1,
      "name": "Dryad Arbor",
      "count": 1
    },
    {
      "op": "addCard",
      "zone": "BATTLEFIELD",
      "player": 1,
      "name": "Runeclaw Bear",
      "count": 1
    },
    {
      "op": "addCard",
      "zone": "BATTLEFIELD",
      "player": 1,
      "name": "Southern Elephant",
      "count": 1
    }
  ]
});
