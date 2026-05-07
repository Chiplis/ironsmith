import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/eld/SyrGwynHeroOfAshvaleTest.java",
  "tests": [
    {
      "name": "equipKnightTest",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Behemoth Sledge",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Syr Gwyn, Hero of Ashvale",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Equip Knight",
          "target": "Syr Gwyn, Hero of Ashvale"
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
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Syr Gwyn, Hero of Ashvale",
          "power": 7,
          "toughness": 7
        }
      ]
    },
    {
      "name": "equipKnightTestInstantSpeed",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Behemoth Sledge",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Leonin Shikari",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Syr Gwyn, Hero of Ashvale",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS",
          "player": 0,
          "ability": "Equip Knight",
          "target": "Syr Gwyn, Hero of Ashvale"
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
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Syr Gwyn, Hero of Ashvale",
          "power": 7,
          "toughness": 7
        }
      ]
    }
  ]
});
