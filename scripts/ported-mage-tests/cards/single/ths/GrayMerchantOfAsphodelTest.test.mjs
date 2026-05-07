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
          "op": "unsupported",
          "source": "checkLife(\"\", 1, PhaseStep.BEGIN_COMBAT, playerB, 18)"
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"\", 1, PhaseStep.BEGIN_COMBAT, playerC, 20)"
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"\", 1, PhaseStep.BEGIN_COMBAT, playerD, 18)"
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"\", 1, PhaseStep.BEGIN_COMBAT, playerA, 24)"
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
      ]
    }
  ]
});
