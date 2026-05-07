import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/triggers/events/UnequipEventTest.java",
  "tests": [
    {
      "name": "testGraftedExoskeletonEvent",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Nazahn, Revered Bladesmith",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Hammer of Nazahn",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 5
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grafted Exoskeleton",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Nazahn, Revered Bladesmith"
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Equip {2}"
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
          "name": "Nazahn, Revered Bladesmith",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Nazahn, Revered Bladesmith",
          "power": 9,
          "toughness": 6
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Nazahn, Revered Bladesmith",
          "ability": "Indestructible",
          "expected": true
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Nazahn, Revered Bladesmith",
          "ability": "Infect",
          "expected": true
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Hammer of Nazahn",
          "count": 1
        }
      ]
    },
    {
      "name": "testGraftedExoskeletonAndBeastWithinEvent",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Nazahn, Revered Bladesmith",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Hammer of Nazahn",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 6
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
          "name": "Beast Within",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Grafted Exoskeleton",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Nazahn, Revered Bladesmith"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Hammer of Nazahn"
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
          "target": "Nazahn, Revered Bladesmith"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Grafted Exoskeleton"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Nazahn, Revered Bladesmith"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Beast Within",
          "target": "Grafted Exoskeleton"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_COMBAT"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "name": "Hammer of Nazahn",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Beast Within",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Beast Token",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Grafted Exoskeleton",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Nazahn, Revered Bladesmith",
          "count": 1
        }
      ]
    }
  ]
});
