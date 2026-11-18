import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/copy/SpelltwineTest.java",
  "tests": [
    {
      "name": "testCopyCards",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Spelltwine",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 1,
          "name": "Shock",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Spelltwine"
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
          "op": "assertExileCount",
          "name": "Spelltwine",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "name": "Shock",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 15
        }
      ]
    },
    {
      "name": "testCopyCardsMirari",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 9
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Spelltwine@spelltwine",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Impulse",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Night's Whisper",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 1,
          "name": "Blasphemous Act",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 1,
          "name": "Divination",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mirari",
          "count": 1
        },
        {
          "op": "castSpell",
          "name": "Spelltwine",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "objectAlias": "@spelltwine",
          "target": {
            "groups": [
              [
                "Impulse"
              ],
              [
                "Blasphemous Act"
              ]
            ]
          }
        },
        {
          "op": "assertStackObject",
          "source": "Spelltwine",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "isSpell": true,
          "targetCount": 2
        },
        {
          "op": "assertStackObject",
          "source": "Mirari",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "abilityKind": "Triggered"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Night's Whisper"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Divination"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "source": "Spelltwine",
          "count": 2,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "isSpell": true,
          "targetCount": 2
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "@spelltwine",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Impulse",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Night's Whisper",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 1,
          "name": "Blasphemous Act",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 1,
          "name": "Divination",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 5
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 18
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
