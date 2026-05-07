import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/mana/MultipleTimesUsableActivatedManaAbilitiesTest.java",
  "tests": [
    {
      "name": "testCanBeCastWithSetonKrosanProtector",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Seton, Krosan Protector",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Citanul Druid",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Leatherback Baloth",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Leatherback Baloth"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Citanul Druid"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Citanul Druid"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Citanul Druid"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertTappedCount",
          "name": "Citanul Druid",
          "tapped": true,
          "count": 3
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Leatherback Baloth",
          "count": 1
        }
      ]
    }
  ]
});
