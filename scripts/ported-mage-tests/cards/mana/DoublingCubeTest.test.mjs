import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/mana/DoublingCubeTest.java",
  "tests": [
    {
      "name": "test_DoublingCubeEldraziTemple",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Eldrazi Temple",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Doubling Cube",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Upwelling",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {G}"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {G}"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {G}"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {C}{C}"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{3}, {T}:"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "op": "unsupported",
          "source": "assertManaPool(playerA, ManaType.COLORLESS, 4)"
        }
      ]
    },
    {
      "name": "test_AvailableMana",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Doubling Cube",
          "count": 1
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
          "player": 0,
          "name": "Mountain",
          "count": 4
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
          "op": "unsupported",
          "source": "assertManaOptions(\"{R}{R}{R}{R}{R}{R}{R}{R}{G}{G}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{R}{R}{R}{R}{R}{R}{G}{G}{G}{G}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{R}{R}{R}{R}{G}{G}{G}{G}{G}{G}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{R}{R}{G}{G}{G}{G}{G}{G}{G}{G}\", manaOptions)"
        }
      ]
    },
    {
      "name": "test_AvailableMana2",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Doubling Cube",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Castle Sengir",
          "count": 2
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
          "player": 0,
          "name": "Mountain",
          "count": 4
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
          "op": "unsupported",
          "source": "assertManaOptions(\"{C}{C}{C}{C}{C}{C}{C}{C}{R}{R}{R}{R}{R}{R}{R}{R}{R}{R}{R}{R}{R}{R}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{C}{C}{C}{C}{C}{C}{R}{R}{R}{R}{R}{R}{R}{R}{R}{R}{R}{R}{R}{R}{R}{R}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{C}{C}{C}{C}{C}{C}{C}{C}{R}{R}{R}{R}{R}{R}{R}{R}{R}{R}{R}{R}{G}{G}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{C}{C}{C}{C}{C}{C}{C}{C}{R}{R}{R}{R}{R}{R}{R}{R}{R}{R}{G}{G}{G}{G}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{C}{C}{C}{C}{C}{C}{C}{C}{R}{R}{R}{R}{R}{R}{R}{R}{G}{G}{G}{G}{G}{G}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{C}{C}{C}{C}{C}{C}{C}{C}{R}{R}{R}{R}{R}{R}{G}{G}{G}{G}{G}{G}{G}{G}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{C}{C}{C}{C}{C}{C}{C}{C}{R}{R}{R}{R}{G}{G}{G}{G}{G}{G}{G}{G}{G}{G}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{C}{C}{C}{C}{C}{C}{C}{C}{R}{R}{G}{G}{G}{G}{G}{G}{G}{G}{G}{G}{G}{G}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{C}{C}{C}{C}{C}{C}{C}{C}{G}{G}{G}{G}{G}{G}{G}{G}{G}{G}{G}{G}{G}{G}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{C}{C}{R}{R}{R}{R}{R}{R}{R}{R}{R}{R}{G}{G}{G}{G}{G}{G}{G}{G}{G}{G}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{C}{C}{C}{C}{B}{B}{R}{R}{G}{G}{G}{G}{G}{G}{G}{G}{G}{G}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{C}{C}{C}{C}{B}{B}{R}{R}{R}{R}{R}{R}{R}{R}{R}{R}{R}{R}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{C}{C}{C}{C}{B}{B}{B}{B}{R}{R}{R}{R}{R}{R}{R}{R}{R}{R}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{C}{C}{C}{C}{B}{B}{R}{R}{R}{R}{R}{R}{R}{R}{R}{R}{G}{G}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{C}{C}{C}{C}{B}{B}{R}{R}{R}{R}{R}{R}{R}{R}{G}{G}{G}{G}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{C}{C}{C}{C}{B}{B}{B}{B}{R}{R}{R}{R}{R}{R}{R}{R}{G}{G}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{C}{C}{C}{C}{B}{B}{B}{B}{R}{R}{R}{R}{R}{R}{G}{G}{G}{G}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{C}{C}{C}{C}{B}{B}{R}{R}{R}{R}{R}{R}{G}{G}{G}{G}{G}{G}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{C}{C}{C}{C}{B}{B}{R}{R}{R}{R}{G}{G}{G}{G}{G}{G}{G}{G}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{C}{C}{C}{C}{B}{B}{B}{B}{R}{R}{R}{R}{G}{G}{G}{G}{G}{G}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{C}{C}{C}{C}{B}{B}{B}{B}{R}{R}{G}{G}{G}{G}{G}{G}{G}{G}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{C}{C}{C}{C}{B}{B}{G}{G}{G}{G}{G}{G}{G}{G}{G}{G}{G}{G}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{C}{C}{C}{C}{B}{B}{B}{B}{G}{G}{G}{G}{G}{G}{G}{G}{G}{G}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{C}{C}{C}{C}{U}{U}{R}{R}{R}{R}{R}{R}{R}{R}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{C}{C}{C}{C}{U}{U}{U}{U}{R}{R}{R}{R}{R}{R}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{C}{C}{C}{C}{G}{G}{U}{U}{R}{R}{R}{R}{R}{R}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{C}{C}{C}{C}{G}{G}{G}{G}{U}{U}{R}{R}{R}{R}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{C}{C}{C}{C}{G}{G}{U}{U}{U}{U}{R}{R}{R}{R}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{C}{C}{C}{C}{G}{G}{G}{G}{U}{U}{U}{U}{R}{R}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{C}{C}{C}{C}{G}{G}{G}{G}{G}{G}{U}{U}{R}{R}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{C}{C}{C}{C}{G}{G}{G}{G}{G}{G}{G}{G}{U}{U}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{C}{C}{C}{C}{G}{G}{G}{G}{G}{G}{U}{U}{U}{U}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{B}{B}{B}{B}{B}{B}{R}{R}{R}{R}{R}{R}{R}{R}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{B}{B}{B}{B}{B}{B}{B}{B}{R}{R}{R}{R}{R}{R}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{B}{B}{B}{B}{B}{B}{R}{R}{R}{R}{R}{R}{G}{G}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{B}{B}{B}{B}{B}{B}{R}{R}{R}{R}{G}{G}{G}{G}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{B}{B}{B}{B}{B}{B}{B}{B}{R}{R}{R}{R}{G}{G}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{B}{B}{B}{B}{B}{B}{B}{B}{R}{R}{G}{G}{G}{G}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{B}{B}{B}{B}{B}{B}{R}{R}{G}{G}{G}{G}{G}{G}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{B}{B}{B}{B}{B}{B}{G}{G}{G}{G}{G}{G}{G}{G}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{B}{B}{B}{B}{B}{B}{B}{B}{G}{G}{G}{G}{G}{G}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{U}{U}{B}{B}{R}{R}{R}{R}{R}{R}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{U}{U}{U}{U}{B}{B}{R}{R}{R}{R}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{U}{U}{B}{B}{B}{B}{R}{R}{R}{R}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{U}{U}{U}{U}{B}{B}{B}{B}{R}{R}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{U}{U}{B}{B}{R}{R}{R}{R}{G}{G}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{U}{U}{B}{B}{R}{R}{G}{G}{G}{G}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{U}{U}{U}{U}{B}{B}{R}{R}{G}{G}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{B}{B}{G}{G}{G}{G}{U}{U}{U}{U}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{U}{U}{B}{B}{B}{B}{R}{R}{G}{G}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{B}{B}{B}{B}{G}{G}{G}{G}{U}{U}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{B}{B}{B}{B}{G}{G}{U}{U}{U}{U}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{B}{B}{G}{G}{G}{G}{G}{G}{U}{U}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{U}{U}{U}{U}{U}{U}\", manaOptions)"
        }
      ]
    }
  ]
});
