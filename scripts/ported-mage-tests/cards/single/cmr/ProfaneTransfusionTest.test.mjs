import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/cmr/ProfaneTransfusionTest.java",
  "tests": [
    {
      "name": "testRegular",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 9
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Profane Transfusion",
          "count": 1
        },
        {
          "op": "setLife",
          "player": 0,
          "life": 24
        },
        {
          "op": "setLife",
          "player": 1,
          "life": 16
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 0
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Profane Transfusion"
        },
        {
          "op": "setStrictChooseMode",
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
          "op": "assertLife",
          "player": 0,
          "life": 16
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 24
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Phyrexian Horror Token",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Phyrexian Horror Token",
          "power": 8,
          "toughness": 8
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Profane Transfusion",
          "count": 1
        }
      ]
    },
    {
      "name": "testCantChange",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 9
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Platinum Emperion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Profane Transfusion",
          "count": 1
        },
        {
          "op": "setLife",
          "player": 0,
          "life": 24
        },
        {
          "op": "setLife",
          "player": 1,
          "life": 16
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 0
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Profane Transfusion"
        },
        {
          "op": "setStrictChooseMode",
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
          "name": "Phyrexian Horror Token",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Phyrexian Horror Token",
          "power": 8,
          "toughness": 8
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Profane Transfusion",
          "count": 1
        }
      ]
    },
    {
      "name": "testDoubleLife",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 9
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Boon Reflection",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Profane Transfusion",
          "count": 1
        },
        {
          "op": "setLife",
          "player": 0,
          "life": 24
        },
        {
          "op": "setLife",
          "player": 1,
          "life": 16
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 0
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Profane Transfusion"
        },
        {
          "op": "setStrictChooseMode",
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
          "op": "assertLife",
          "player": 0,
          "life": 16
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 32
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Phyrexian Horror Token",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Phyrexian Horror Token",
          "power": 16,
          "toughness": 16
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Profane Transfusion",
          "count": 1
        }
      ]
    },
    {
      "name": "testCantGainLife",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Badlands",
          "count": 11
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Skullcrack",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Profane Transfusion",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Skullcrack",
          "target": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 0
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Profane Transfusion"
        },
        {
          "op": "setStrictChooseMode",
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Phyrexian Horror Token",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Phyrexian Horror Token",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Profane Transfusion",
          "count": 1
        }
      ]
    }
  ]
});
