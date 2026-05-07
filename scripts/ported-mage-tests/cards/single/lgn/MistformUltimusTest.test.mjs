import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/lgn/MistformUltimusTest.java",
  "tests": [
    {
      "name": "testMistformUltimus",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mistform Ultimus",
          "count": 1
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
          "source": "assertSubtype(ultimus, SubType.GOBLIN)"
        }
      ]
    },
    {
      "name": "testGoblinChieftain",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mistform Ultimus",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Goblin Chieftain",
          "count": 1
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
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Mistform Ultimus",
          "power": 4,
          "toughness": 4
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Mistform Ultimus",
          "ability": "Haste",
          "expected": true
        }
      ]
    }
  ]
});
