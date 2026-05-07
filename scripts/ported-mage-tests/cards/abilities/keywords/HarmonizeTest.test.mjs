import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/keywords/HarmonizeTest.java",
  "tests": [
    {
      "name": "testNoTap",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 7
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Channeled Dragonfire",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Harmonize",
          "target": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": false
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
          "op": "unsupported",
          "source": "assertTapped(bear, false)"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 18
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Channeled Dragonfire",
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "name": "Mountain",
          "tapped": true,
          "count": 7
        }
      ]
    },
    {
      "name": "testTap",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Channeled Dragonfire",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Harmonize",
          "target": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Grizzly Bears"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Grizzly Bears"
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
          "op": "unsupported",
          "source": "assertTapped(bear, true)"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 18
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Channeled Dragonfire",
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "name": "Mountain",
          "tapped": true,
          "count": 5
        }
      ]
    }
  ]
});
