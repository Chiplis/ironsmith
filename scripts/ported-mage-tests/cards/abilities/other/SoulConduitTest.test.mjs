import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/other/SoulConduitTest.java",
  "tests": [
    {
      "name": "basicTwoPlayerGame_SoulConduitEffect",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 9
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Soul Conduit",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Live Fast",
          "count": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Live Fast"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "ability": "{6}"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 0
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Live Fast",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": 0,
          "counter": "ENERGY",
          "count": 2
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": 1,
          "counter": "ENERGY",
          "count": 0
        },
        {
          "op": "unsupported",
          "source": "assertTapped(soulConduit, true)"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 18
        }
      ]
    }
  ]
});
