import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/acr/TheAesirEscapeValhallaTest.java",
  "tests": [
    {
      "name": "test_SimplePlay",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "The Aesir Escape Valhalla",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Gigantosaurus",
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
          "name": "The Aesir Escape Valhalla"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Gigantosaurus"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "The Aesir Escape Valhalla",
          "count": 1
        },
        {
          "op": "assertLife",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "life": "20 + 5",
          "seatIndependent": true
        },
        {
          "op": "assertExileCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Gigantosaurus",
          "count": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Memnite"
        },
        {
          "op": "assertCounterCount",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Memnite",
          "counter": "P1P1",
          "count": 5
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Gigantosaurus",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "The Aesir Escape Valhalla",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Gigantosaurus",
          "count": 0
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "The Aesir Escape Valhalla",
          "count": 0
        }
      ]
    }
  ]
});
