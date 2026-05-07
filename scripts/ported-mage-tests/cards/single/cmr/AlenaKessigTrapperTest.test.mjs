import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/cmr/AlenaKessigTrapperTest.java",
  "tests": [
    {
      "name": "test_alena",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Alena, Kessig Trapper",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Frost Walker",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Barbarian Horde",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 2
        },
        {
          "op": "assertPlayableAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Barbarian Horde",
          "expected": false
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Frost Walker"
        },
        {
          "op": "assertPlayableAbility",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "label": "Cast Barbarian Horde",
          "expected": true
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Barbarian Horde"
        },
        {
          "op": "assertPlayableAbility",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Barbarian Horde",
          "expected": false
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Barbarian Horde",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Barbarian Horde",
          "count": 1
        }
      ]
    }
  ]
});
