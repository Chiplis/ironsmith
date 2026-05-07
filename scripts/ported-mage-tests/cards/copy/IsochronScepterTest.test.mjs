import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/copy/IsochronScepterTest.java",
  "tests": [
    {
      "name": "testImprint",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Isochron Scepter",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Isochron Scepter"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Yes"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Lightning Bolt"
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
          "name": "Isochron Scepter",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        }
      ]
    },
    {
      "name": "testCopyCard",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Isochron Scepter",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Isochron Scepter"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Lightning Bolt"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{2}, {T}:"
        },
        {
          "op": "setChoice",
          "player": 0,
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Isochron Scepter",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 0
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 17
        }
      ]
    },
    {
      "name": "testCopyCardButDontCast",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Isochron Scepter",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Isochron Scepter"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Lightning Bolt"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{2}, {T}:"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": false
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Isochron Scepter",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 0
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        }
      ]
    },
    {
      "name": "testAngelsGrace",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Isochron Scepter",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Angel's Grace",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Dross Crocodile",
          "count": 4
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
          "name": "Mountain",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Isochron Scepter"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Angel's Grace"
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Dross Crocodile",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Dross Crocodile",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Dross Crocodile",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Dross Crocodile",
          "defender": 1
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "DECLARE_BLOCKERS",
          "player": 0,
          "ability": "{2}, {T}:"
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
          "op": "castSpell",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Lightning Bolt",
          "target": 0
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Lightning Bolt",
          "target": "Dross Crocodile"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 1
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Lightning Bolt",
          "count": 2
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Dross Crocodile",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Dross Crocodile",
          "count": 3
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Isochron Scepter",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Angel's Grace",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Angel's Grace",
          "count": 0
        }
      ]
    },
    {
      "name": "testSilence",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Isochron Scepter",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Silence",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Plains",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Isochron Scepter"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Silence"
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "UPKEEP",
          "player": 0,
          "ability": "{2}, {T}:"
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
          "op": "assertPlayableAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "label": "Cast Silvercoat",
          "expected": false
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Isochron Scepter",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Silence",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 0
        }
      ]
    }
  ]
});
