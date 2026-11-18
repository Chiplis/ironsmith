import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/continuous/PaintersServantTest.java",
  "tests": [
    {
      "name": "testColorSet",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Painter's Servant",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lightning Bolt",
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
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Lightning Bolt",
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
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Painter's Servant"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Blue"
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
          "name": "Painter's Servant",
          "count": 1
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "library",
            "player": 0
          },
          "checks": [
            {
              "property": "color",
              "value": "blue",
              "expected": true
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "library",
            "player": 1
          },
          "checks": [
            {
              "property": "color",
              "value": "blue",
              "expected": true
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "hand",
            "player": 0
          },
          "checks": [
            {
              "property": "color",
              "value": "red",
              "expected": true
            },
            {
              "property": "color",
              "value": "blue",
              "expected": true
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "hand",
            "player": 1
          },
          "checks": [
            {
              "property": "color",
              "value": "red",
              "expected": true
            },
            {
              "property": "color",
              "value": "blue",
              "expected": true
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "graveyard",
            "player": 0
          },
          "checks": [
            {
              "property": "color",
              "value": "white",
              "expected": true
            },
            {
              "property": "color",
              "value": "blue",
              "expected": true
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "graveyard",
            "player": 1
          },
          "checks": [
            {
              "property": "color",
              "value": "white",
              "expected": true
            },
            {
              "property": "color",
              "value": "blue",
              "expected": true
            }
          ]
        }
      ]
    },
    {
      "name": "testColorReset",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Painter's Servant",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lightning Bolt",
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
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Lightning Bolt",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Painter's Servant"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Blue"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Lightning Bolt",
          "target": "Painter's Servant"
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
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Painter's Servant",
          "count": 1
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "library",
            "player": 0
          },
          "checks": [
            {
              "property": "color",
              "value": "blue",
              "expected": false
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "library",
            "player": 1
          },
          "checks": [
            {
              "property": "color",
              "value": "blue",
              "expected": false
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "hand",
            "player": 0
          },
          "checks": [
            {
              "property": "color",
              "value": "red",
              "expected": true
            },
            {
              "property": "color",
              "value": "blue",
              "expected": false
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "hand",
            "player": 1
          },
          "checks": [
            {
              "property": "color",
              "value": "red",
              "expected": true
            },
            {
              "property": "color",
              "value": "blue",
              "expected": false
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "graveyard",
            "player": 0,
            "name": "Silvercoat Lion"
          },
          "checks": [
            {
              "property": "color",
              "value": "white",
              "expected": true
            },
            {
              "property": "color",
              "value": "blue",
              "expected": false
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "graveyard",
            "player": 1,
            "name": "Silvercoat Lion"
          },
          "checks": [
            {
              "property": "color",
              "value": "white",
              "expected": true
            },
            {
              "property": "color",
              "value": "blue",
              "expected": false
            }
          ]
        }
      ]
    },
    {
      "name": "testColorOverwrite",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Painter's Servant",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 2
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
          "player": 1,
          "name": "Cerulean Wisps",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Painter's Servant"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Red"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Cerulean Wisps",
          "target": "Silvercoat Lion",
          "waitForStack": "WHILE_NOT_ON_STACK"
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
          "name": "Painter's Servant",
          "count": 1
        }
      ]
    },
    {
      "name": "testColorSpell",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Painter's Servant",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Divination",
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
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Dragon's Claw",
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
          "player": 1,
          "name": "Cerulean Wisps",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Painter's Servant"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Red"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Cerulean Wisps",
          "target": "Silvercoat Lion",
          "waitForStack": "WHILE_NOT_ON_STACK"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Divination",
          "target": "TestPlayer.NO_TARGET",
          "waitForStack": "WHILE_NOT_ON_STACK"
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
          "name": "Painter's Servant",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Divination",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Cerulean Wisps",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 22
        }
      ]
    },
    {
      "name": "testColorSpellEnds",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Painter's Servant",
          "count": 1
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
          "name": "Pyroblast",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Green Sun's Zenith",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Altar's Light",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Plains",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Ambush Viper",
          "count": 2
        },
        {
          "op": "castSpell",
          "name": "Painter's Servant",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertStackObject",
          "source": "Painter's Servant",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "isSpell": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Blue"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Painter's Servant",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "X=2"
        },
        {
          "op": "castSpell",
          "name": "Green Sun's Zenith",
          "player": 1,
          "turn": 2,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertStackObject",
          "source": "Green Sun's Zenith",
          "count": 1,
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "isSpell": true,
          "xValue": 2
        },
        {
          "op": "setModeChoice",
          "player": 0,
          "value": 1
        },
        {
          "op": "castSpell",
          "name": "Pyroblast",
          "player": 0,
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "holdStack": true,
          "target": "Green Sun's Zenith"
        },
        {
          "op": "assertStackObject",
          "source": "Pyroblast",
          "count": 1,
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "isSpell": true,
          "objectTarget": "Green Sun's Zenith"
        },
        {
          "op": "assertStackObject",
          "source": "Green Sun's Zenith",
          "count": 1,
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "isSpell": true
        },
        {
          "op": "castSpell",
          "name": "Altar's Light",
          "player": 1,
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "holdStack": true,
          "target": "Painter's Servant"
        },
        {
          "op": "assertStackObject",
          "source": "Altar's Light",
          "count": 1,
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "isSpell": true,
          "objectTarget": "Painter's Servant",
          "onTop": true
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Painter's Servant",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Altar's Light",
          "count": 1
        },
        {
          "op": "assertStackObject",
          "source": "Pyroblast",
          "count": 1,
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "isSpell": true,
          "onTop": true
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Pyroblast",
          "count": 1
        },
        {
          "op": "assertStackObject",
          "source": "Green Sun's Zenith",
          "count": 1,
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "isSpell": true,
          "onTop": true
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Ambush Viper"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Ambush Viper",
          "count": 1
        },
        {
          "op": "assertLibraryCount",
          "player": 1,
          "name": "Green Sun's Zenith",
          "count": 1
        }
      ]
    }
  ]
});
