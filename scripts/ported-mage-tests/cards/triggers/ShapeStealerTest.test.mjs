import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/triggers/ShapeStealerTest.java",
  "tests": [
    {
      "name": "testShapeStealerSingleBlocker",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Shape Stealer",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Myojin of Cleansing Fire",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Shape Stealer",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Myojin of Cleansing Fire",
          "attacker": "Shape Stealer"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerA, shapeStealer, 4)"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Shape Stealer",
          "power": 4,
          "toughness": 6
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerB, myojinOfCleansingFire, 4)"
        }
      ]
    }
  ]
});
