import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/m12/GrandAbolisherTest.java",
  "tests": [
    {
      "name": "test_MakeSureItWorksFromBattlefieldOnly",
      "operations": [
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Grand Abolisher",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Plains",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Consider",
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
          "op": "assertHandCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Grand Abolisher",
          "count": 0
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "label": "Cast Consider",
          "expected": true
        },
        {
          "op": "assertHandCount",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Grand Abolisher",
          "count": 1
        },
        {
          "op": "assertPlayableAbility",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "label": "Cast Consider",
          "expected": true
        },
        {
          "op": "assertHandCount",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Grand Abolisher",
          "count": 1
        },
        {
          "op": "assertPlayableAbility",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "label": "Cast Consider",
          "expected": true
        },
        {
          "op": "castSpell",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Grand Abolisher"
        },
        {
          "op": "waitStackResolved",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "assertPermanentCount",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Grand Abolisher",
          "count": 1
        },
        {
          "op": "assertPlayableAbility",
          "turn": 4,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "label": "Cast Consider",
          "expected": false
        },
        {
          "op": "assertPermanentCount",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Grand Abolisher",
          "count": 1
        },
        {
          "op": "assertPlayableAbility",
          "turn": 5,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "label": "Cast Consider",
          "expected": true
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        }
      ]
    }
  ]
});
