import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/keywords/SpliceOnArcaneTest.java",
  "tests": [
    {
      "name": "testSpliceThroughTheBreach",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lava Spike",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Through the Breach",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Lava Spike",
          "target": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Through the Breach"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Silvercoat Lion"
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
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 17
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Lava Spike",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Through the Breach",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Silvercoat Lion",
          "ability": "Haste",
          "expected": true
        },
        {
          "op": "assertManaOptions",
          "player": 0,
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "options": [
            ""
          ],
          "exact": true
        }
      ]
    },
    {
      "name": "testSpliceTorrentOfStone",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lava Spike",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Torrent of Stone",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Lava Spike",
          "target": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Mountain"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Torrent of Stone"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Silvercoat Lion"
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
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 17
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Lava Spike",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Torrent of Stone",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Mountain",
          "count": 0
        },
        {
          "op": "assertManaOptions",
          "player": 0,
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "options": [
            ""
          ],
          "exact": true
        }
      ]
    },
    {
      "name": "testSpliceThroughTheBreach2",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Nourishing Shoal",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Giant Growth",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Through the Breach",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Nourishing Shoal",
          "actionLabel": "Cast Nourishing Shoal (parsed alternative cost)"
        },
        {
          "op": "setObjectChoices",
          "player": 0,
          "description": "Reveal cards to splice onto Nourishing Shoal, in resolution order",
          "choices": [
            "Through the Breach"
          ]
        },
        {
          "op": "assertChoiceMenu",
          "kind": "number",
          "player": 0,
          "description": "Choose value for X",
          "value": 1
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "description": "Choose the next cost to pay for Nourishing Shoal",
          "kind": "options",
          "choices": [
            "Exile a green card from hand"
          ]
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Giant Growth"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Nourishing Shoal",
          "count": 1,
          "xValue": 1
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Giant Growth",
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "player": 0,
          "name": "Mountain",
          "tapped": true,
          "count": 4
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Silvercoat Lion"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
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
          "op": "assertLife",
          "player": 0,
          "life": 21
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Nourishing Shoal",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Through the Breach",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Silvercoat Lion",
          "ability": "Haste",
          "expected": true
        },
        {
          "op": "assertManaOptions",
          "player": 0,
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "options": [
            ""
          ],
          "exact": true
        }
      ]
    },
    {
      "name": "testCounteredBecauseOfNoLegalTarget",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "name": "Forest",
          "count": 2,
          "player": 0
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "name": "Swamp",
          "count": 8,
          "player": 0
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "name": "Nourishing Shoal",
          "count": 1,
          "player": 0
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "name": "Goryo's Vengeance@splice",
          "count": 1,
          "player": 0
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "name": "Goryo's Vengeance@response",
          "count": 1,
          "player": 0
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "name": "Griselbrand",
          "count": 1,
          "player": 0
        },
        {
          "op": "setOptionChoices",
          "description": "Choose casting method for Nourishing Shoal",
          "choices": [
            "Normal: {X}{G}{G}"
          ],
          "kind": "options",
          "player": 0
        },
        {
          "op": "setObjectChoices",
          "description": "Reveal cards to splice onto Nourishing Shoal, in resolution order",
          "choices": [
            {
              "alias": "@splice"
            }
          ],
          "player": 0
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=3"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Nourishing Shoal",
          "target": {
            "groups": [
              [
                "Griselbrand"
              ]
            ]
          },
          "holdStack": true
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Nourishing Shoal",
          "count": 1,
          "isSpell": true,
          "objectTarget": "Griselbrand",
          "targetCount": 1,
          "xValue": 3
        },
        {
          "op": "setObjectChoices",
          "description": "Reveal cards to splice onto Goryo's Vengeance, in resolution order",
          "choices": [],
          "player": 0
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Goryo's Vengeance",
          "target": "Griselbrand",
          "holdStack": true,
          "objectAlias": "@response"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Nourishing Shoal",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Goryo's Vengeance",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Goryo's Vengeance",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Griselbrand",
          "count": 1
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
        }
      ]
    }
  ]
});
