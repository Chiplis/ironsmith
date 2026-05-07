import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/m21/DubTest.java",
  "tests": [
    {
      "name": "testBoostAndAbilities",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Dub",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Scryb Sprites",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Dub",
          "target": "Scryb Sprites"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Scryb Sprites",
          "ability": "FirstStrike",
          "expected": true
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Scryb Sprites",
          "ability": "Flying",
          "expected": true
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Scryb Sprites",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(\"Scryb Sprites\", SubType.KNIGHT)"
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(\"Scryb Sprites\", SubType.FAERIE)"
        }
      ]
    }
  ]
});
