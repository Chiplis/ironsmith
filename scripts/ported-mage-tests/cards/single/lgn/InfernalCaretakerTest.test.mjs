import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/lgn/InfernalCaretakerTest.java",
  "tests": [
    {
      "name": "testInfernalCaretaker",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 10
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Walking Corpse",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Infernal Caretaker",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Walking Corpse",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Storm Crow",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 1,
          "name": "Festering Goblin",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 1,
          "name": "Elvish Visionary",
          "count": 4
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Infernal Caretaker using Morph"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{3}{B}: Turn this face-down permanent face up."
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
          "name": "Infernal Caretaker",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Walking Corpse",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Walking Corpse",
          "count": 4
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Storm Crow",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Festering Goblin",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Elvish Visionary",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 4
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Infernal Caretaker",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Walking Corpse",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "name": "Walking Corpse",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "name": "Storm Crow",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "name": "Festering Goblin",
          "count": 4
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "name": "Elvish Visionary",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "count": 4
        }
      ]
    }
  ]
});
