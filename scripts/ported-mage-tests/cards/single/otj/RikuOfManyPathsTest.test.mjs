import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/otj/RikuOfManyPathsTest.java",
  "tests": [
    {
      "name": "testModeChoice",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Underground Sea",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Riku of Many Paths",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Artful Takedown",
          "count": 1
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "modes",
          "description": "Choose mode for Artful Takedown",
          "choices": [
            "Tap target creature.",
            "Target creature gets -2/-4 until end of turn."
          ]
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "options",
          "description": "Choose 0-3 modes",
          "choices": [
            "Put a +1/+1 counter on Riku. It gains trample until end of turn.",
            "Create a 1/1 blue Bird creature token with flying."
          ]
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Artful Takedown",
          "target": {
            "groups": [
              [
                "Grizzly Bears"
              ],
              [
                "Silvercoat Lion"
              ]
            ]
          }
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Artful Takedown",
          "count": 1,
          "targetCount": 2
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Riku of Many Paths",
          "count": 1,
          "abilityKind": "Triggered",
          "onTop": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 2
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
          "op": "assertTapped",
          "name": "Grizzly Bears",
          "tapped": true,
          "player": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Riku of Many Paths",
          "counter": "P1P1",
          "count": 1
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Riku of Many Paths",
          "ability": "Trample",
          "expected": true
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Bird Token",
          "count": 1
        }
      ]
    }
  ]
});
