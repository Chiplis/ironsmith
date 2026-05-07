import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/c19/AeonEngineTest.java",
  "tests": [
    {
      "name": "testEnterTappedNormalTurnOrder",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Aeon Engine",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": "playerD",
          "name": "Agonizing Syphon",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerD",
          "name": "Swamp",
          "count": 4
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Aeon Engine"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": "Agonizing Syphon",
          "target": 0
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Aeon Engine\", true)"
        },
        {
          "op": "unsupported",
          "source": "assertActivePlayer(playerD)"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 17
        },
        {
          "op": "assertLife",
          "player": "playerD",
          "life": 23
        }
      ]
    }
  ]
});
