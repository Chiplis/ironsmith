import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/dsk/UnableToScreamTest.java",
  "tests": [
    {
      "name": "preventFromTurningFaceUpTest",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Akroma, Angel of Fury",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Appetite for the Unnatural",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Forest",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Unable to Scream",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Break Open",
          "count": 2
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Akroma, Angel of Fury using Morph"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Unable to Scream",
          "target": "EmptyNames.FACE_DOWN_CREATURE.getTestCommand()"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Break Open",
          "target": "EmptyNames.FACE_DOWN_CREATURE.getTestCommand()"
        },
        {
          "op": "assertPermanentCount",
          "turn": 3,
          "phase": "BEGIN_COMBAT",
          "player": 1,
          "name": "EmptyNames.FACE_DOWN_CREATURE.getTestCommand()",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 3,
          "phase": "BEGIN_COMBAT",
          "player": 1,
          "name": "Akroma, Angel of Fury",
          "count": 0
        },
        {
          "op": "castSpell",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Appetite for the Unnatural",
          "target": "Unable to Scream"
        },
        {
          "op": "castSpell",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Break Open",
          "target": "EmptyNames.FACE_DOWN_CREATURE.getTestCommand()"
        },
        {
          "op": "assertPermanentCount",
          "turn": 5,
          "phase": "BEGIN_COMBAT",
          "player": 1,
          "name": "EmptyNames.FACE_DOWN_CREATURE.getTestCommand()",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "turn": 5,
          "phase": "BEGIN_COMBAT",
          "player": 1,
          "name": "Akroma, Angel of Fury",
          "count": 1
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        }
      ]
    },
    {
      "name": "becomesEffectTest",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Appetite for the Unnatural",
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
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Forest",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Unable to Scream",
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
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Unable to Scream",
          "target": "Llanowar Elves"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Appetite for the Unnatural",
          "target": "Unable to Scream"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        }
      ]
    }
  ]
});
