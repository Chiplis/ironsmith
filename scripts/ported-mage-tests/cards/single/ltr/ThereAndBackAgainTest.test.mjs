import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/ltr/ThereAndBackAgainTest.java",
  "tests": [
    {
      "name": "test_SimplePlay",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "There and Back Again",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Volcanic Island",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gaea's Protector",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Amaranthine Wall",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "There and Back Again"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Amaranthine Wall"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Gaea's Protector",
          "defender": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Mountain"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Gaea's Protector",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 5,
          "player": 0,
          "attacker": "Gaea's Protector",
          "defender": 1
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
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": "20 - 4 * 2"
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerB, \"Amaranthine Wall\", 4)"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Smaug",
          "count": 1
        }
      ]
    }
  ]
});
