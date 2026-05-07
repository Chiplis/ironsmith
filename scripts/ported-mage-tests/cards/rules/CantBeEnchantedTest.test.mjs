import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/rules/CantBeEnchantedTest.java",
  "tests": [
    {
      "name": "testConsecrateLand",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Consecrate Land",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Psychic Venom",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Consecrate Land",
          "target": "Plains"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Psychic Venom",
          "target": "Plains"
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
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Consecrate Land",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Consecrate Land",
          "count": 1
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Plains",
          "ability": "Indestructible",
          "expected": true
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Psychic Venom",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Psychic Venom",
          "count": 1
        }
      ]
    },
    {
      "name": "testConsecrateLandEnchantedBefore",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Consecrate Land",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Psychic Venom",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Psychic Venom",
          "target": "Plains"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Consecrate Land",
          "target": "Plains"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Consecrate Land",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Consecrate Land",
          "count": 1
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Plains",
          "ability": "Indestructible",
          "expected": true
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 18
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Psychic Venom",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Psychic Venom",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "name": "Psychic Venom",
          "count": 0
        }
      ]
    }
  ]
});
