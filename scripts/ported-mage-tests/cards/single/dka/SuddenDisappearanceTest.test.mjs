import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/dka/SuddenDisappearanceTest.java",
  "tests": [
    {
      "name": "testCard",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Sudden Disappearance",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Air Elemental",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Horned Turtle",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Altar of the Lost",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Sudden Disappearance",
          "target": 1
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
          "player": 1,
          "name": "Air Elemental",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Horned Turtle",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Altar of the Lost",
          "count": 0
        },
        {
          "op": "assertExileCount",
          "name": "Air Elemental",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "name": "Horned Turtle",
          "count": 4
        },
        {
          "op": "assertExileCount",
          "name": "Altar of the Lost",
          "count": 1
        }
      ]
    },
    {
      "name": "testCard1",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Sudden Disappearance",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Air Elemental",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Horned Turtle",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Altar of the Lost",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Sudden Disappearance",
          "target": 1
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "UPKEEP"
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
          "player": 1,
          "name": "Air Elemental",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Horned Turtle",
          "count": 4
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Altar of the Lost",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "name": "Air Elemental",
          "count": 0
        },
        {
          "op": "assertExileCount",
          "name": "Horned Turtle",
          "count": 0
        },
        {
          "op": "assertExileCount",
          "name": "Altar of the Lost",
          "count": 0
        }
      ]
    }
  ]
});
