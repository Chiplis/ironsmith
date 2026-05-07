import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/ogw/EldraziObligatorTest.java",
  "tests": [
    {
      "name": "targetCreatureDoNotPayAdditionalCost",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Bronze Sable",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Eldrazi Obligator",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 3
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Bronze Sable",
          "defender": 1
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Eldrazi Obligator"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": false
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Bronze Sable",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Eldrazi Obligator",
          "count": 1
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Bronze Sable",
          "ability": "Haste",
          "expected": false
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Bronze Sable\", true)"
        }
      ]
    }
  ]
});
