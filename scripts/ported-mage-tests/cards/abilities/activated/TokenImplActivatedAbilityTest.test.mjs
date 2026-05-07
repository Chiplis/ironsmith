import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/activated/TokenImplActivatedAbilityTest.java",
  "tests": [
    {
      "name": "testActivatedManaAbility",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Omnath, Locus of Mana",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Freyalise, Llanowar's Fury",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "+2: Create a 1/1 green Elf Druid creature token with \"{T}: Add {G}.\""
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "+2: Create a 1/1 green Elf Druid creature token with \"{T}: Add {G}.\""
        },
        {
          "op": "activateManaAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {G}.",
          "count": 1
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Elf Druid Token",
          "count": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Freyalise, Llanowar's Fury",
          "count": 1
        }
      ]
    }
  ]
});
