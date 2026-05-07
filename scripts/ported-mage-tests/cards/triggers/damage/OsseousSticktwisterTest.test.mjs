import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/triggers/damage/OsseousSticktwisterTest.java",
  "tests": [
    {
      "name": "testDelirium",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Osseous Sticktwister",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Priest of Titania",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Llanowar Elves",
          "count": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "UNTAP"
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
          "op": "assertPermanentCount",
          "player": 0,
          "count": 0,
          "name": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "count": 1,
          "name": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 1,
          "name": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 0,
          "name": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 1,
          "name": 0
        }
      ]
    },
    {
      "name": "testDiscard",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Osseous Sticktwister",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Priest of Titania",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Llanowar Elves",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Sheoldred",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Dark Ritual",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Damn",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Phyrexian Arena",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": false
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Priest of Titania"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "UNTAP"
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
          "op": "assertPermanentCount",
          "player": 0,
          "count": 0,
          "name": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "count": 1,
          "name": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 1,
          "name": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 0,
          "name": 4
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 1,
          "name": 1
        }
      ]
    },
    {
      "name": "testSacrifice",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Osseous Sticktwister",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Priest of Titania",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Llanowar Elves",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Sheoldred",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Dark Ritual",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Damn",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Phyrexian Arena",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Llanowar Elves"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "UNTAP"
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
          "op": "assertPermanentCount",
          "player": 0,
          "count": 0,
          "name": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "count": 1,
          "name": 0
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 1,
          "name": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 0,
          "name": 4
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 1,
          "name": 1
        }
      ]
    },
    {
      "name": "testDamage",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Osseous Sticktwister",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Priest Of Titania",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Llanowar Elves",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Sheoldred",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Dark Ritual",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Damn",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Phyrexian Arena",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": false
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "UNTAP"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 22
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 18
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "count": 0,
          "name": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "count": 1,
          "name": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 1,
          "name": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 0,
          "name": 4
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 1,
          "name": 0
        }
      ]
    },
    {
      "name": "testModifiedDamage",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Osseous Sticktwister",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Unholy Strength",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Priest Of Titania",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Llanowar Elves",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Sheoldred",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Dark Ritual",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Damn",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Phyrexian Arena",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Unholy Strength",
          "target": "Osseous Sticktwister"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": false
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "UNTAP"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 24
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 16
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "count": 0,
          "name": 3
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "count": 1,
          "name": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 1,
          "name": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 0,
          "name": 4
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 1,
          "name": 0
        }
      ]
    }
  ]
});
