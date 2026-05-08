import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/afc/IndomitableMightTest.java",
  "tests": [
    {
      "name": "testAsThoughEffect",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Runeclaw Bear",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Centaur Courser",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Azure Drake",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Fortress Crab",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Indomitable Might",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Indomitable Might",
          "target": "Runeclaw Bear"
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Runeclaw Bear",
          "power": 5,
          "toughness": 5
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Runeclaw Bear",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Centaur Courser",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Azure Drake",
          "attacker": "Centaur Courser"
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Fortress Crab",
          "attacker": "Runeclaw Bear"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
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
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 15
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerA, bear, 1)"
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerA, centaur, 2)"
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerB, drake, 3)"
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerB, crab, 0)"
        }
      ]
    }
  ]
});
