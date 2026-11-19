import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/other/ChiefEngineerTest.java",
  "tests": [
    {
      "name": "testGainsConvoke",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Chief Engineer",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Alpha Myr",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "name": "Cathodion",
          "count": 1,
          "player": 0
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Goblin Roughrider",
          "count": 1
        },
        {
          "op": "assertCastMethods",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Goblin Roughrider",
          "methods": []
        },
        {
          "op": "setManaPayment",
          "player": 0,
          "source": "Cathodion",
          "alternatives": [
            {
              "source": "Alpha Myr",
              "kind": "convoke"
            }
          ]
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Cathodion"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Cathodion",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Goblin Roughrider",
          "count": 1
        },
        {
          "op": "assertTapped",
          "player": 0,
          "name": "Alpha Myr",
          "tapped": true
        }
      ]
    }
  ]
});
