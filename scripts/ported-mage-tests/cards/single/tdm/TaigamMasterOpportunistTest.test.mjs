import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/tdm/TaigamMasterOpportunistTest.java",
  "tests": [
    {
      "name": "testCardWithSpellOption",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Taigam, Master Opportunist",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Yawgmoth's Bargain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Ornithopter",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Twinmaw Stormbrood",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plateau",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Aegis Turtle",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Ornithopter",
          "resolve": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Charring Bite",
          "target": "Aegis Turtle"
        },
        {
          "op": "assertEachObject",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "selector": {
            "zone": "exile",
            "name": "Twinmaw Stormbrood",
            "player": 0
          },
          "count": 1,
          "checks": [
            {
              "property": "counter",
              "value": "TIME",
              "expected": 4
            }
          ]
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Cast Charring Bite"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Aegis Turtle"
        },
        {
          "op": "setStopAt",
          "turn": 9,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLibraryCount",
          "player": 0,
          "name": "Twinmaw Stormbrood",
          "count": 1,
          "note": "CR 720.3d: Charring Bite cast as an Omen from suspend (CR 702.62a) is shuffled into its owner's library as it resolves, as the XMage original asserts."
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Aegis Turtle",
          "count": 2
        }
      ]
    },
    {
      "name": "testMDFC",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Taigam, Master Opportunist",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plateau",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Ornithopter",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Akoum Warrior@warrior",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Ornithopter",
          "holdStack": true
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Ornithopter",
          "count": 1,
          "isSpell": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Akoum Warrior",
          "holdStack": true,
          "objectAlias": "@warrior"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Akoum Warrior",
          "count": 1,
          "isSpell": true
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Taigam, Master Opportunist",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "@warrior",
          "count": 1
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "exile",
            "player": 0,
            "alias": "@warrior"
          },
          "count": 1,
          "checks": [
            {
              "property": "counter",
              "value": "time",
              "expected": 4
            }
          ]
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Play Akoum Teeth"
        },
        {
          "op": "waitStackResolved",
          "turn": 9,
          "phase": "UPKEEP"
        },
        {
          "op": "waitStackResolved",
          "turn": 9,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Akoum Teeth",
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "player": 0,
          "name": "Akoum Teeth",
          "tapped": true,
          "count": 1
        }
      ]
    },
    {
      "name": "testMDFC2",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plateau",
          "count": 6
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
          "zone": "HAND",
          "player": 1,
          "name": "Delay",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Akoum Warrior@warrior",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Akoum Warrior",
          "holdStack": true,
          "objectAlias": "@warrior"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Akoum Warrior",
          "count": 1,
          "isSpell": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Delay",
          "holdStack": true,
          "target": {
            "groups": [
              [
                "@warrior"
              ]
            ]
          }
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Delay",
          "count": 1,
          "isSpell": true,
          "objectTarget": "@warrior"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "@warrior",
          "count": 1
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "exile",
            "player": 0,
            "alias": "@warrior"
          },
          "count": 1,
          "checks": [
            {
              "property": "counter",
              "value": "time",
              "expected": 3
            }
          ]
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Play Akoum Teeth"
        },
        {
          "op": "waitStackResolved",
          "turn": 7,
          "phase": "UPKEEP"
        },
        {
          "op": "waitStackResolved",
          "turn": 7,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Akoum Teeth",
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "player": 0,
          "name": "Akoum Teeth",
          "tapped": true,
          "count": 1
        }
      ]
    },
    {
      "name": "test",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plateau",
          "count": 6
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
          "player": 1,
          "name": "Aegis Turtle",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Delay",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Twinmaw Stormbrood",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Twinmaw Stormbrood"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Delay",
          "target": "Twinmaw Stormbrood"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Cast Charring Bite"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Aegis Turtle"
        },
        {
          "op": "setStopAt",
          "turn": 7,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        }
      ]
    }
  ]
});
