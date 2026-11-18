import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/ltr/BillFernyBreeSwindlerTest.java",
  "tests": [
    {
      "name": "giveAHorseTest",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Bill Ferny, Bree Swindler",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Armored Warhorse",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Horseshoe Crab",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Iron Golem",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Bill Ferny, Bree Swindler",
          "defender": 1
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "options",
          "description": "Choose a mode",
          "choices": [
            "Target opponent gains control of target Horse you control. If they do, remove Bill Ferny from combat and create three Treasure tokens."
          ]
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Armored Warhorse"
        },
        {
          "op": "assertBlockDeclarationRejected",
          "turn": 1,
          "player": 1,
          "reason": "must block",
          "declarations": []
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Iron Golem",
          "attacker": "Bill Ferny, Bree Swindler"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Armored Warhorse",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Bill Ferny, Bree Swindler",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Treasure Token",
          "count": 3
        }
      ]
    },
    {
      "name": "phantasmalImageTest",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Bill Ferny, Bree Swindler",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Armored Warhorse",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Horseshoe Crab",
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
          "name": "Phantasmal Image@horse.image",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Iron Golem",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Phantasmal Image"
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "options",
          "description": "Choose what to copy as this permanent enters",
          "choices": [
            "Enter as a copy of Armored Warhorse"
          ]
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Bill Ferny, Bree Swindler",
          "defender": 1
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "options",
          "description": "Choose a mode",
          "choices": [
            "Target opponent gains control of target Horse you control. If they do, remove Bill Ferny from combat and create three Treasure tokens."
          ]
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "@horse.image"
        },
        {
          "op": "assertBlockDeclarationRejected",
          "turn": 1,
          "player": 1,
          "reason": "must block",
          "declarations": []
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Iron Golem",
          "attacker": "Bill Ferny, Bree Swindler"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Armored Warhorse",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Bill Ferny, Bree Swindler",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Treasure Token",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Armored Warhorse",
          "count": 1
        }
      ]
    }
  ]
});
