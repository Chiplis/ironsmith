import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/spm/AlienSymbiosisTest.java",
  "tests": [
    {
      "name": "testAlienSymbiosisCastFromGrave",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Alien Symbiosis",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Bear Cub",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Alien Symbiosis",
          "target": "Bear Cub"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Bear Cub",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "unsupported",
          "source": "assertAbilityCount(playerA, bearCub, MenaceAbility.class, 1)"
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(bearCub, SubType.SYMBIOTE)"
        }
      ]
    }
  ]
});
