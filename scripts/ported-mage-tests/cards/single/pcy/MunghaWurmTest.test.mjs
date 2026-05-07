import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/pcy/MunghaWurmTest.java",
  "tests": [
    {
      "name": "wurmEffect",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mungha Wurm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Alpine Meadow",
          "count": 10
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Azorius Guildgate",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Golgari Guildgate",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Arctic Treeline",
          "count": 10
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Azorius Guildgate"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Golgari Guildgate"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "UPKEEP"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertTappedCount",
          "name": "Alpine Meadow",
          "tapped": true,
          "count": 10
        },
        {
          "op": "assertTappedCount",
          "name": "Azorius Guildgate",
          "tapped": false,
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "name": "Golgari Guildgate",
          "tapped": false,
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "name": "Arctic Treeline",
          "tapped": false,
          "count": 10
        }
      ]
    },
    {
      "name": "wurmDying",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Doom Blade",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mungha Wurm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Alpine Meadow",
          "count": 10
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Azorius Guildgate",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Arctic Treeline",
          "count": 10
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Azorius Guildgate"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "END_TURN",
          "player": 0,
          "name": "Doom Blade",
          "target": "Mungha Wurm"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "UPKEEP"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertTappedCount",
          "name": "Alpine Meadow",
          "tapped": false,
          "count": 10
        },
        {
          "op": "assertTappedCount",
          "name": "Azorius Guildgate",
          "tapped": false,
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "name": "Arctic Treeline",
          "tapped": false,
          "count": 10
        }
      ]
    }
  ]
});
