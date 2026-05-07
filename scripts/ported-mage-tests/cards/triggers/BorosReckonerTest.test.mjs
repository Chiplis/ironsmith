import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/triggers/BorosReckonerTest.java",
  "tests": [
    {
      "name": "testDamageAmountLikeDamageDealt",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Phytotitan",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Boros Reckoner",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Boros Reckoner",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 2,
          "player": 0,
          "blocker": "Phytotitan",
          "attacker": "Boros Reckoner"
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": 0
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 13
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Phytotitan",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Phytotitan\", true)"
        }
      ]
    }
  ]
});
