import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/cmm/DemonOfFatesDesignTest.java",
  "tests": [
    {
      "name": "CastForLife",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Demon of Fate's Design",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Glorious Anthem",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Glorious Anthem"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Cast with alternative cost: Pay 3 life"
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
          "name": "Glorious Anthem",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 17
        }
      ]
    },
    {
      "name": "SayNoToTheDemon",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Demon of Fate's Design",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Glorious Anthem",
          "count": 1
        },
        {
          "op": "assertCastMethods",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Glorious Anthem",
          "methods": [
            "play_from"
          ],
          "priceSource": "Demon of Fate's Design",
          "methodLabel": "pay life equal to mana value"
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
          "op": "assertHandCount",
          "player": 0,
          "name": "Glorious Anthem",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        }
      ]
    },
    {
      "name": "CantCastBothInSameTurn",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Demon of Fate's Design",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Glorious Anthem",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Absolute Law",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Glorious Anthem"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Cast with alternative cost: Pay 3 life"
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "label": "Cast Absolute Law",
          "expected": false
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
          "name": "Glorious Anthem",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 17
        }
      ]
    },
    {
      "name": "CantCastDuringOpponentTurn",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Demon of Fate's Design",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Dictate of Kruphix",
          "count": 1
        },
        {
          "op": "assertPlayableAbility",
          "turn": 2,
          "phase": "UPKEEP",
          "player": 0,
          "label": "Cast Dictate of Kruphix",
          "expected": false
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "DRAW"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        }
      ]
    },
    {
      "name": "CantCastIfOpponentHasDemon",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Demon of Fate's Design",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Glorious Anthem",
          "count": 1
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Glorious Anthem",
          "expected": false
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
          "op": "assertHandCount",
          "player": 0,
          "name": "Glorious Anthem",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        }
      ]
    },
    {
      "name": "CanCastBothInDifferentTurn",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Demon of Fate's Design",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Glorious Anthem",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Absolute Law",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Glorious Anthem"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Cast with alternative cost: Pay 3 life"
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
          "name": "Glorious Anthem",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 17
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Absolute Law"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Cast with alternative cost: Pay 2 life"
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
          "name": "Absolute Law",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 15
        }
      ]
    },
    {
      "name": "BlinkTest",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Demon of Fate's Design",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Cloudshift",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Glorious Anthem",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Absolute Law",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Glorious Anthem",
          "resolve": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Cast with alternative cost: Pay 3 life"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Cloudshift",
          "target": "Demon of Fate's Design",
          "resolve": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Absolute Law"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Cast with alternative cost: Pay 2 life"
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
          "name": "Glorious Anthem",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Absolute Law",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 15
        }
      ]
    },
    {
      "name": "UnsubstantiateTest",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Demon of Fate's Design",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Unsubstantiate",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Glorious Anthem",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Glorious Anthem"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Cast with alternative cost: Pay 3 life"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Unsubstantiate",
          "target": "Glorious Anthem"
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Glorious Anthem",
          "expected": false
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
          "op": "assertHandCount",
          "player": 0,
          "name": "Glorious Anthem",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 17
        }
      ]
    },
    {
      "name": "DoubleDemonDoubleCast",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Demon of Fate's Design",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Sakashima the Impostor",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Glorious Anthem",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Absolute Law",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Sakashima the Impostor"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Sakashima the Impostor",
          "count": 1
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "options",
          "description": "Choose what to copy as this permanent enters",
          "choices": [
            "Enter as a copy of Demon of Fate's Design"
          ]
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Sakashima the Impostor",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Demon of Fate's Design",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Glorious Anthem",
          "castingMethod": "play_from",
          "priceSource": "Sakashima the Impostor"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Glorious Anthem",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 17
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Glorious Anthem",
          "count": 1
        },
        {
          "op": "assertCastMethods",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Absolute Law",
          "methods": [
            "play_from"
          ],
          "priceSource": "Demon of Fate's Design"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Absolute Law",
          "castingMethod": "play_from",
          "priceSource": "Demon of Fate's Design"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Absolute Law",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 15
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Absolute Law",
          "count": 1
        }
      ]
    },
    {
      "name": "DoubleDemonDoubleCast2",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Demon of Fate's Design",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Sakashima the Impostor",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Glorious Anthem",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Absolute Law",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Sakashima the Impostor"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Sakashima the Impostor",
          "count": 1
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "options",
          "description": "Choose what to copy as this permanent enters",
          "choices": [
            "Enter as a copy of Demon of Fate's Design"
          ]
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Sakashima the Impostor",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Demon of Fate's Design",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Glorious Anthem",
          "castingMethod": "play_from",
          "priceSource": "Demon of Fate's Design"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Glorious Anthem",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 17
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Glorious Anthem",
          "count": 1
        },
        {
          "op": "assertCastMethods",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Absolute Law",
          "methods": [
            "play_from"
          ],
          "priceSource": "Sakashima the Impostor"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Absolute Law",
          "castingMethod": "play_from",
          "priceSource": "Sakashima the Impostor"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Absolute Law",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 15
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Absolute Law",
          "count": 1
        }
      ]
    },
    {
      "name": "testNoCastFromGraveyard",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Demon of Fate's Design",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Glorious Anthem",
          "count": 1
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Glorious Anthem",
          "expected": false
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        }
      ]
    },
    {
      "name": "testYesCastFromGraveyard",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Demon of Fate's Design",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Glorious Anthem@anthem",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Yawgmoth's Will",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Yawgmoth's Will"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Yawgmoth's Will",
          "count": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertCastMethods",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "owner": 0,
          "zone": "graveyard",
          "name": "Glorious Anthem",
          "objectAlias": "@anthem",
          "methods": [
            "alternative_price"
          ],
          "priceSource": "Demon of Fate's Design",
          "note": "Yawgmoth's Will supplies the graveyard permission and Demon's life payment is a separate alternative cost for that cast (CR 118.9, 601.2b); Ironsmith names that pairing alternative_price."
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Glorious Anthem",
          "castingMethod": "alternative_price",
          "priceSource": "Demon of Fate's Design",
          "objectAlias": "@anthem"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Glorious Anthem",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 17
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Glorious Anthem",
          "count": 1
        }
      ]
    },
    {
      "name": "testCanCastTransforming",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Demon of Fate's Design",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "The Legend of Kyoshi",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "The Legend of Kyoshi"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Cast with alternative cost: Pay 6 life"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "The Legend of Kyoshi",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 14
        }
      ]
    }
  ]
});
