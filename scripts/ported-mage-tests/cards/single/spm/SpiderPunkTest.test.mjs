import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/spm/SpiderPunkTest.java",
  "tests": [
    {
      "name": "testSpiderPunk",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Damage Fixture",
          "custom": true,
          "fixtureSource": true,
          "typeLine": "",
          "oracleText": "{T}: Damage Fixture deals 2 damage to target player."
        },
        {
          "op": "addCardWithEntryChoice",
          "player": 0,
          "name": "Spider-Punk",
          "choice": "This creature gains haste"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Giant Spider",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Counterspell",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Disallow",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Giant Spider"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Counterspell",
          "target": "Giant Spider"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "This creature gains haste"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Giant Spider",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Spider-Punk",
          "defender": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: This permanent deals 2 damage to target player",
          "target": 1,
          "source": "Damage Fixture"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Disallow"
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "stack ability"
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
          "op": "assertLife",
          "player": 1,
          "life": 14
        },
        {
          "op": "assertKeywordCount",
          "player": 0,
          "name": "Giant Spider",
          "ability": "RiotAbility.class",
          "count": 1
        }
      ]
    }
  ]
});
