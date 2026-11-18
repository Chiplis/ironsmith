import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/woc/CourtOfLocthwainTest.java",
  "tests": [
    {
      "name": "testNoMonarch",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Court of Locthwain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Scrubland",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Cabal Evangel",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Falkenrath Reaver@court.1",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Island",
          "count": 2
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Court of Locthwain"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertMonarch",
          "monarch": 0
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Cabal Evangel",
          "defender": 0
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "DRAW"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertMonarch",
          "monarch": 1
        },
        {
          "op": "assertExileCount",
          "name": "@court.1",
          "count": 1,
          "player": 1
        },
        {
          "op": "assertPlayableAbility",
          "player": 0,
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "label": "Cast Falkenrath Reaver",
          "actionLabel": "Cast Falkenrath Reaver (from exile)",
          "expected": true
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Falkenrath Reaver",
          "objectAlias": "@court.1",
          "actionLabel": "Cast Falkenrath Reaver (from exile)",
          "permissionSource": "Court of Locthwain"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Falkenrath Reaver",
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "name": "Scrubland",
          "tapped": true,
          "count": 2
        }
      ]
    },
    {
      "name": "testMonarchChoiceCastForFree",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Court of Locthwain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Scrubland",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Falkenrath Reaver@court.1",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Island",
          "count": 1
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Court of Locthwain"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertMonarch",
          "monarch": 0
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertExileCount",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "@court.1",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertPlayableAbility",
          "player": 0,
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "label": "Cast Falkenrath Reaver",
          "actionLabel": "Cast Falkenrath Reaver (without paying its mana cost, from exile)",
          "expected": true
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Falkenrath Reaver",
          "objectAlias": "@court.1",
          "actionLabel": "Cast Falkenrath Reaver (without paying its mana cost, from exile)",
          "permissionSource": "Court of Locthwain"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Falkenrath Reaver",
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "name": "Scrubland",
          "tapped": true,
          "count": 0
        }
      ]
    },
    {
      "name": "testMonarchChoiceCastMDFCForFree",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Court of Locthwain",
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
          "name": "Swamp",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Fire // Ice@court.1",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Island",
          "count": 1
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Court of Locthwain"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertMonarch",
          "monarch": 0
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertExileCount",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "@court.1",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertPlayableAbility",
          "player": 0,
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "label": "Cast Ice",
          "actionLabel": "Cast Ice (without paying its mana cost, from exile)",
          "expected": true
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Ice",
          "target": "Grizzly Bears",
          "objectAlias": "@court.1",
          "actionLabel": "Cast Ice (without paying its mana cost, from exile)",
          "permissionSource": "Court of Locthwain"
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
          "op": "assertTappedCount",
          "name": "Swamp",
          "tapped": true,
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Fire // Ice",
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "name": "Grizzly Bears",
          "tapped": true,
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 3
        }
      ]
    },
    {
      "name": "testMonarchChoiceCastForMana",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Court of Locthwain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Scrubland",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Falkenrath Reaver@court.1",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Island",
          "count": 1
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Court of Locthwain"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertMonarch",
          "monarch": 0
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertExileCount",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "@court.1",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertPlayableAbility",
          "player": 0,
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "label": "Cast Falkenrath Reaver",
          "actionLabel": "Cast Falkenrath Reaver (from exile)",
          "expected": true
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Falkenrath Reaver",
          "objectAlias": "@court.1",
          "actionLabel": "Cast Falkenrath Reaver (from exile)",
          "permissionSource": "Court of Locthwain"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Falkenrath Reaver",
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "name": "Scrubland",
          "tapped": true,
          "count": 2
        }
      ]
    },
    {
      "name": "testMonarchArmageddon",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Court of Locthwain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Armageddon",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Scrubland",
          "count": 8
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Falkenrath Reaver@court.1",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Cabal Evangel@court.2",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Island",
          "count": 1
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Court of Locthwain"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertMonarch",
          "monarch": 0
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Armageddon"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertExileCount",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "@court.2",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertExileCount",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "@court.1",
          "count": 0,
          "seatIndependent": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertExileCount",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "@court.2",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertExileCount",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "@court.1",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertPlayableAbility",
          "player": 0,
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "label": "Cast Falkenrath Reaver",
          "actionLabel": "Cast Falkenrath Reaver (without paying its mana cost, from exile)",
          "expected": true
        },
        {
          "op": "castSpell",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Falkenrath Reaver",
          "objectAlias": "@court.1",
          "actionLabel": "Cast Falkenrath Reaver (without paying its mana cost, from exile)",
          "permissionSource": "Court of Locthwain"
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Falkenrath Reaver",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Scrubland",
          "count": 0
        }
      ]
    },
    {
      "name": "testMonarchDoubleCast",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Court of Locthwain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Scrubland",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Falkenrath Reaver@court.1",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Cabal Evangel@court.2",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Island",
          "count": 1
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Court of Locthwain"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertMonarch",
          "monarch": 0
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertExileCount",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "@court.2",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertExileCount",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "@court.1",
          "count": 0,
          "seatIndependent": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertExileCount",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "@court.2",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertExileCount",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "@court.1",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertPlayableAbility",
          "player": 0,
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "label": "Cast Cabal Evangel",
          "actionLabel": "Cast Cabal Evangel (from exile)",
          "expected": true
        },
        {
          "op": "castSpell",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Cabal Evangel",
          "objectAlias": "@court.2",
          "actionLabel": "Cast Cabal Evangel (from exile)",
          "permissionSource": "Court of Locthwain"
        },
        {
          "op": "assertPlayableAbility",
          "player": 0,
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "label": "Cast Falkenrath Reaver",
          "actionLabel": "Cast Falkenrath Reaver (without paying its mana cost, from exile)",
          "expected": true
        },
        {
          "op": "castSpell",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Falkenrath Reaver",
          "objectAlias": "@court.1",
          "actionLabel": "Cast Falkenrath Reaver (without paying its mana cost, from exile)",
          "permissionSource": "Court of Locthwain"
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Cabal Evangel",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Falkenrath Reaver",
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "name": "Scrubland",
          "tapped": true,
          "count": 2
        }
      ]
    }
  ]
});
