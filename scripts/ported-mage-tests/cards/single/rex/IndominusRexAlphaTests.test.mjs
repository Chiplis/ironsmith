import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/rex/IndominusRexAlphaTests.java",
  "tests": [
    {
      "name": "testIndominusRexAlphaAllAbilties",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Indominus Rex, Alpha",
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
          "name": "Rograkh, Son of Rohgahh",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Adorned Pouncer",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Ankle Biter",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Gladecover Scout",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Banehound",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Bontu the Glorified",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Aerial Responder",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Stonecoil Serpent",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Codespell Cleric",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Swamp",
          "count": 20
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
          "name": "Forest",
          "count": 3
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Indominus Rex, Alpha"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Ornithopter^Rograkh, Son of Rohgahh^Adorned Pouncer^Ankle Biter^Gladecover Scout^Banehound^Bontu the Glorified^Aerial Responder^Stonecoil Serpent^Codespell Cleric"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Indominus Rex, Alpha",
          "counter": "FLYING",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Indominus Rex, Alpha",
          "counter": "FIRST_STRIKE",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Indominus Rex, Alpha",
          "counter": "DOUBLE_STRIKE",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Indominus Rex, Alpha",
          "counter": "DEATHTOUCH",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Indominus Rex, Alpha",
          "counter": "HEXPROOF",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Indominus Rex, Alpha",
          "counter": "HASTE",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Indominus Rex, Alpha",
          "counter": "INDESTRUCTIBLE",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Indominus Rex, Alpha",
          "counter": "LIFELINK",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Indominus Rex, Alpha",
          "counter": "MENACE",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Indominus Rex, Alpha",
          "counter": "REACH",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Indominus Rex, Alpha",
          "counter": "TRAMPLE",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Indominus Rex, Alpha",
          "counter": "VIGILANCE",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 12
        }
      ]
    },
    {
      "name": "testIndominusRexAlphaHexproofFromX",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Indominus Rex, Alpha",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Eradicator Valkyrie",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Swamp",
          "count": 20
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
          "name": "Forest",
          "count": 3
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Indominus Rex, Alpha"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Eradicator Valkyrie"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Indominus Rex, Alpha",
          "counter": "FLYING",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Indominus Rex, Alpha",
          "counter": "HEXPROOF",
          "count": 0
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Indominus Rex, Alpha",
          "counter": "HexproofFromPlaneswalkersAbility.getInstance().getRule()",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Indominus Rex, Alpha",
          "counter": "LIFELINK",
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
      "name": "testIndominusRexAlphaGraveyardMovement",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Rest in Peace",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Indominus Rex, Alpha",
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
          "name": "Rograkh, Son of Rohgahh",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Adorned Pouncer",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Ankle Biter",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Gladecover Scout",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Banehound",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Bontu the Glorified",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Aerial Responder",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Stonecoil Serpent",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Codespell Cleric",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Swamp",
          "count": 20
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
          "name": "Forest",
          "count": 3
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Indominus Rex, Alpha"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Ornithopter^Rograkh, Son of Rohgahh^Adorned Pouncer^Ankle Biter^Gladecover Scout^Banehound^Bontu the Glorified^Aerial Responder^Stonecoil Serpent^Codespell Cleric"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Indominus Rex, Alpha",
          "counter": "FLYING",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Indominus Rex, Alpha",
          "counter": "FIRST_STRIKE",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Indominus Rex, Alpha",
          "counter": "DOUBLE_STRIKE",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Indominus Rex, Alpha",
          "counter": "DEATHTOUCH",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Indominus Rex, Alpha",
          "counter": "HEXPROOF",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Indominus Rex, Alpha",
          "counter": "HASTE",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Indominus Rex, Alpha",
          "counter": "INDESTRUCTIBLE",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Indominus Rex, Alpha",
          "counter": "LIFELINK",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Indominus Rex, Alpha",
          "counter": "MENACE",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Indominus Rex, Alpha",
          "counter": "REACH",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Indominus Rex, Alpha",
          "counter": "TRAMPLE",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Indominus Rex, Alpha",
          "counter": "VIGILANCE",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 12
        }
      ]
    },
    {
      "name": "testIndominusRexAlphaSubset",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Indominus Rex, Alpha",
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
          "name": "Rograkh, Son of Rohgahh",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Adorned Pouncer",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Ankle Biter",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Banehound",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Bontu the Glorified",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Aerial Responder",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Codespell Cleric",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Swamp",
          "count": 20
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
          "name": "Forest",
          "count": 3
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Indominus Rex, Alpha"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Ornithopter^Rograkh, Son of Rohgahh^Adorned Pouncer^Ankle Biter^Banehound^Bontu the Glorified^Aerial Responder^Codespell Cleric"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Indominus Rex, Alpha",
          "counter": "FLYING",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Indominus Rex, Alpha",
          "counter": "FIRST_STRIKE",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Indominus Rex, Alpha",
          "counter": "DOUBLE_STRIKE",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Indominus Rex, Alpha",
          "counter": "DEATHTOUCH",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Indominus Rex, Alpha",
          "counter": "HASTE",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Indominus Rex, Alpha",
          "counter": "INDESTRUCTIBLE",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Indominus Rex, Alpha",
          "counter": "LIFELINK",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Indominus Rex, Alpha",
          "counter": "MENACE",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Indominus Rex, Alpha",
          "counter": "TRAMPLE",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Indominus Rex, Alpha",
          "counter": "VIGILANCE",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 10
        }
      ]
    },
    {
      "name": "testIndominusRexAlphaDiscardReplacement",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Indominus Rex, Alpha",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Nullhide Ferox",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Swamp",
          "count": 20
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
          "name": "Forest",
          "count": 3
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Indominus Rex, Alpha"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Nullhide Ferox"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Indominus Rex, Alpha",
          "counter": "HEXPROOF",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Nullhide Ferox",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Nullhide Ferox",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 1
        }
      ]
    },
    {
      "name": "testIndominusRexAlphaMadness",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Indominus Rex, Alpha",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Kitchen Imp",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Swamp",
          "count": 20
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 3
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Indominus Rex, Alpha"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Kitchen Imp"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "When this card"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Yes"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Indominus Rex, Alpha",
          "counter": "FLYING",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Indominus Rex, Alpha",
          "counter": "HASTE",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Kitchen Imp",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Kitchen Imp",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 2
        }
      ]
    }
  ]
});
