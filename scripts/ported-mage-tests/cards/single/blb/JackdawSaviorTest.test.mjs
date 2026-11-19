import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/blb/JackdawSaviorTest.java",
  "tests": [
    {
      "name": "test_Simultaneous",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Jackdaw Savior",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Air Elemental",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Damnation",
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
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Damnation",
          "holdStack": true
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Damnation",
          "count": 1,
          "isSpell": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Whenever {this} or another creature you control"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Jackdaw Savior"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Memnite"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Jackdaw Savior",
          "count": 2,
          "abilityKind": "Triggered",
          "completeTargets": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
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
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Jackdaw Savior",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Memnite",
          "count": 1
        }
      ]
    },
    {
      "name": "test_Clones",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Jackdaw Savior",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Air Elemental",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Spidersilk Armor",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Phantasmal Image@first",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Phantasmal Image@second",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Murder",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Underground Sea",
          "count": 10
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Phantasmal Image",
          "objectAlias": "@first"
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "options",
          "description": "Choose what to copy as this permanent enters",
          "choices": [
            "Enter as a copy of Air Elemental"
          ]
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
          "name": "Murder"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "@first"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "@first",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Phantasmal Image",
          "objectAlias": "@second"
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "options",
          "description": "Choose what to copy as this permanent enters",
          "choices": [
            "Enter as a copy of Air Elemental"
          ]
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
          "name": "Murder"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "@second"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "@first"
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "options",
          "description": "Choose what to copy as this permanent enters",
          "choices": [
            "Don't copy"
          ]
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
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Murder",
          "count": 2
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Phantasmal Image",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Phantasmal Image",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Jackdaw Savior",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Air Elemental",
          "count": 1
        }
      ]
    }
  ]
});
