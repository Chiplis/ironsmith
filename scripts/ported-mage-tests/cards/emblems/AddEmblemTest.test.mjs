import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/emblems/AddEmblemTest.java",
  "tests": [
    {
      "name": "test_ElpethSunChampionEmblem",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "unsupported",
          "source": "addEmblem(playerA, elspethSunChampionEmblem)"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "UPKEEP"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Grizzly Bears",
          "power": 4,
          "toughness": 4
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Grizzly Bears",
          "ability": "Flying",
          "expected": true
        },
        {
          "op": "unsupported",
          "source": "assertCommandZoneCount(playerA, elspethSunChampionEmblem.getName(), 1)"
        }
      ]
    },
    {
      "name": "test_ChandraTorchOfDefianceEmblem",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "unsupported",
          "source": "addEmblem(playerA, chandraTorchOfDefianceEmblem)"
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Memnite",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Memnite"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Memnite"
        },
        {
          "op": "addTarget",
          "player": 0,
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Memnite",
          "count": 2
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 10
        },
        {
          "op": "unsupported",
          "source": "assertCommandZoneCount(playerA, chandraTorchOfDefianceEmblem.getName(), 1)"
        }
      ]
    }
  ]
});
