import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/lci/EatenByPiranhasTest.java",
  "tests": [
    {
      "name": "testEatenByPiranhas",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Eaten by Piranhas",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Eaten by Piranhas"
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "Balduvian Bears"
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
          "op": "unsupported",
          "source": "assertType(\"Balduvian Bears\", CardType.CREATURE, SubType.SKELETON)"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Balduvian Bears",
          "power": 1,
          "toughness": 1
        },
        {
          "op": "unsupported",
          "source": "assertColor(playerA, \"Balduvian Bears\", ObjectColor.BLACK, true)"
        }
      ]
    }
  ]
});
