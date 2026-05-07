import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/m21/DemonicEmbraceTest.java",
  "tests": [
    {
      "name": "playFromGraveyard",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Demonic Embrace",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Demonic Embrace",
          "target": "Grizzly Bears"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Mountain"
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
          "player": 0,
          "name": "Demonic Embrace",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Grizzly Bears",
          "power": 5,
          "toughness": 3
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 17
        }
      ]
    }
  ]
});
