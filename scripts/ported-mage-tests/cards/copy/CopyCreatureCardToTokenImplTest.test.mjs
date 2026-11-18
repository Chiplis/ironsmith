import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/copy/CopyCreatureCardToTokenImplTest.java",
  "tests": [
    {
      "name": "testTokenTriggeresETBEffect",
      "operations": [
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Unesh, Criosphinx Sovereign",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Hour of Eternity",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 5
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Hour of Eternity",
          "target": "Unesh, Criosphinx Sovereign"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=1"
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
          "life": 20
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Hour of Eternity",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Unesh, Criosphinx Sovereign",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Unesh, Criosphinx Sovereign",
          "count": 0
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Unesh, Criosphinx Sovereign",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 3
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 2
        }
      ],
      "skip": "upstream @Ignore"
    },
    {
      "name": "testFaerieArtisans",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Faerie Artisans",
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
          "player": 1,
          "name": "Thrashing Brontodon@native",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Forest",
          "count": 4
        },
        {
          "op": "castSpell",
          "name": "Thrashing Brontodon",
          "player": 1,
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "objectAlias": "@native"
        },
        {
          "op": "assertStackObject",
          "source": "Thrashing Brontodon",
          "count": 1,
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "isSpell": true
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "source": "Faerie Artisans",
          "count": 1,
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Thrashing Brontodon",
            "token": true
          },
          "count": 1,
          "checks": [
            {
              "property": "cardType",
              "value": "artifact",
              "expected": true
            },
            {
              "property": "cardType",
              "value": "creature",
              "expected": true
            }
          ]
        },
        {
          "op": "activateAbility",
          "source": "Thrashing Brontodon",
          "ability": "Destroy target artifact or enchantment",
          "player": 1,
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "target": "Alpha Myr"
        },
        {
          "op": "assertStackObject",
          "source": "Thrashing Brontodon",
          "count": 1,
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "abilityKind": "Activated",
          "objectTarget": "Alpha Myr"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "@native",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Alpha Myr",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Thrashing Brontodon",
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
    },
    {
      "name": "testTokenCopyTransformedHasSecondFaceWithModifications",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Baithook Angler",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Faerie Artisans",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Breeding Pool",
          "count": 5
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Cast Hook-Haunt Drifter using Disturb"
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
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Hook-Haunt Drifter",
            "token": false
          },
          "count": 1,
          "checks": [
            {
              "property": "cardType",
              "value": "Creature",
              "expected": true
            },
            {
              "property": "doubleFaced",
              "expected": true
            },
            {
              "property": "transformed",
              "expected": true
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 1,
            "token": true,
            "name": "Hook-Haunt Drifter"
          },
          "checks": [
            {
              "property": "cardType",
              "expected": true,
              "value": "Artifact"
            },
            {
              "property": "doubleFaced",
              "expected": true
            },
            {
              "property": "transformed",
              "expected": true
            }
          ],
          "count": 1
        }
      ]
    }
  ]
});
