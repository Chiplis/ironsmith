import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/other/ImpelledGiantTest.java",
  "tests": [
    {
      "name": "testGainsPower",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Impelled Giant",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Hurloon Minotaur",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Tap an untapped red creature you control other than"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Hurloon Minotaur"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Hurloon Minotaur\", true)"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Impelled Giant",
          "power": 5,
          "toughness": 3
        }
      ]
    }
  ]
});
