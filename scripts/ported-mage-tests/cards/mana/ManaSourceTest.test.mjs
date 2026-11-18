import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/mana/ManaSourceTest.java",
  "tests": [
    {
      "name": "testCantCastWithCreatureCard",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Simian Spirit Guide",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Myr Superion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Manakin",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Exile",
          "count": 1,
          "source": "Simian Spirit Guide"
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Manakin",
          "ability": "{T}: Add {C}"
        },
        {
          "op": "assertManaPool",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "colors": "RC",
          "amount": 2
        },
        {
          "op": "assertCastMethods",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Myr Superion",
          "methods": [],
          "zone": "hand"
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
          "op": "assertExileCount",
          "player": 0,
          "name": "Simian Spirit Guide",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Myr Superion",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Myr Superion",
          "count": 1
        }
      ]
    }
  ]
});
