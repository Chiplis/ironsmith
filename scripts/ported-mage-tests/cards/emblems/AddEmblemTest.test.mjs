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
          "op": "addEmblem",
          "player": 0,
          "name": "Elspeth, Sun's Champion Emblem",
          "oracleText": "Creatures you control get +2/+2 and have flying."
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
          "op": "assertEmblemCount",
          "player": 0,
          "name": "Elspeth, Sun's Champion Emblem",
          "count": 1
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
          "op": "addEmblem",
          "player": 0,
          "name": "Chandra, Torch of Defiance Emblem",
          "oracleText": "Whenever you cast a spell, this emblem deals 5 damage to any target."
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
          "phase": "PRECOMBAT_MAIN"
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
          "op": "assertEmblemCount",
          "player": 0,
          "name": "Chandra, Torch of Defiance Emblem",
          "count": 1
        }
      ]
    }
  ]
});
