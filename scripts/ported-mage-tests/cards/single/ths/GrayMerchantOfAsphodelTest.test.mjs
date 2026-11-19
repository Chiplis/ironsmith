import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/ths/GrayMerchantOfAsphodelTest.java",
  "tests": [
    {
      "name": "testDevotionLifeDrain",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Gray Merchant of Asphodel",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 10
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Gray Merchant of Asphodel"
        },
        {
          "op": "assertLife",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 1,
          "life": "18",
          "seatIndependent": true
        },
        {
          "op": "assertLife",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 2,
          "life": "20",
          "seatIndependent": true
        },
        {
          "op": "assertLife",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 3,
          "life": "18",
          "seatIndependent": true
        },
        {
          "op": "assertLife",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "life": "24",
          "seatIndependent": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Gray Merchant of Asphodel"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "UPKEEP"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 14
        },
        {
          "op": "assertLife",
          "player": "playerC",
          "life": 20
        },
        {
          "op": "assertLife",
          "player": "playerD",
          "life": 14
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 32
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Gray Merchant of Asphodel",
          "count": 2
        }
      ],
      "rangeOfInfluence": 1,
      "turnOrder": [
        0,
        3,
        2,
        1
      ]
    }
  ]
});
