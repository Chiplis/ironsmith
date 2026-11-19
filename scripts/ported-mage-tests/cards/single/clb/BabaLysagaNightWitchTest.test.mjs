import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/clb/BabaLysagaNightWitchTest.java",
  "tests": [
    {
      "name": "SacrificeAnimatedMishra",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Baba Lysaga, Night Witch",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mishra's Factory",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "source": "Mishra's Factory",
          "ability": "Add {C}"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "ability": "becomes a 2/2",
          "source": "Mishra's Factory"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "UPKEEP"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Mishra's Factory"
          },
          "count": 1,
          "checks": [
            {
              "property": "cardType",
              "value": "Land",
              "expected": true
            },
            {
              "property": "cardType",
              "value": "Artifact",
              "expected": true
            },
            {
              "property": "cardType",
              "value": "Creature",
              "expected": true
            }
          ]
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Sacrifice up to three",
          "source": "Baba Lysaga, Night Witch"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Mishra's Factory"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "TestPlayer.CHOICE_SKIP"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 23
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 17
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 3
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Mishra's Factory",
          "count": 1
        }
      ]
    },
    {
      "name": "SacrificeNonAnimatedMishra",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Baba Lysaga, Night Witch",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mishra's Factory",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Sacrifice up to three",
          "source": "Baba Lysaga, Night Witch"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Mishra's Factory"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "TestPlayer.CHOICE_SKIP"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
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
          "life": 20
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Mishra's Factory",
          "count": 1
        }
      ]
    }
  ]
});
