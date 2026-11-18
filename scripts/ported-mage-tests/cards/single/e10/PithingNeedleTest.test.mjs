import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/e10/PithingNeedleTest.java",
  "tests": [
    {
      "name": "TestPithingNeedle",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Pithing Needle",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Pillarfield Ox",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Proteus Staff",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerC",
          "name": "Island",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerC",
          "name": "Proteus Staff",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerC",
          "name": "Wall of Air",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": "playerC",
          "name": "Wind Drake",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerD",
          "name": "Island",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerD",
          "name": "Eager Cadet",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": "playerD",
          "name": "Storm Crow",
          "count": 2
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Pithing Needle"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Proteus Staff"
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerC",
          "ability": "{2}{U}",
          "target": "Eager Cadet",
          "source": "Proteus Staff"
        },
        {
          "op": "assertPermanentCount",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Pithing Needle",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertNoAction",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "kind": "activate_ability",
          "source": "Proteus Staff",
          "label": "{2}{U},",
          "checkPayment": true
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
          "op": "assertPermanentCount",
          "player": 3,
          "name": "Eager Cadet",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 3,
          "name": "Storm Crow",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 2,
          "name": "Wall of Air",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 2,
          "name": "Wind Drake",
          "count": 0
        }
      ],
      "turnOrder": [
        0,
        3,
        2,
        1
      ],
      "rangeOfInfluence": 1
    }
  ]
});
