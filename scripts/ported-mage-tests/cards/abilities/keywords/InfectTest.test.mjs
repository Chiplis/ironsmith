import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/keywords/InfectTest.java",
  "tests": [
    {
      "name": "testNormalUse",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Tine Shrike",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Tine Shrike",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": 0,
          "counter": "POISON",
          "count": 2
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        }
      ]
    },
    {
      "name": "testLoseInfectUse",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Melira, Sylvok Outcast",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Tine Shrike",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Tine Shrike",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": 0,
          "counter": "POISON",
          "count": 0
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 18
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        }
      ]
    },
    {
      "name": "testInkmothNexusLoseInfect",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Melira, Sylvok Outcast",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Virulent Wound",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Inkmoth Nexus",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Virulent Wound",
          "target": "Melira, Sylvok Outcast"
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "ability": "{1}: {this} becomes"
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Inkmoth Nexus",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Virulent Wound",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Melira, Sylvok Outcast",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Plains\", true)"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Inkmoth Nexus\", true)"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": 0,
          "counter": "POISON",
          "count": 0
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        }
      ]
    },
    {
      "name": "testInkmothPumpedByBecomeImmense1",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 7
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Inkmoth Nexus",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Become Immense",
          "count": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{1}: {this} becomes"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Inkmoth Nexus",
          "defender": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS",
          "player": 0,
          "name": "Become Immense"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Inkmoth Nexus"
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
          "op": "unsupported",
          "source": "assertTapped(\"Inkmoth Nexus\", true)"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Become Immense",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": 1,
          "counter": "POISON",
          "count": 7
        }
      ]
    },
    {
      "name": "testInkmothPumpedByBecomeImmense2",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 7
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Inkmoth Nexus",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Become Immense",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Mutagenic Growth",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Might of Old Krosa",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{1}: {this} becomes"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Inkmoth Nexus",
          "defender": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS",
          "player": 0,
          "name": "Mutagenic Growth"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS",
          "player": 0,
          "name": "Might of Old Krosa"
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{1}: {this} becomes"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Inkmoth Nexus",
          "defender": 1
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "DECLARE_ATTACKERS",
          "player": 0,
          "name": "Become Immense"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Inkmoth Nexus\", true)"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Become Immense",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Mutagenic Growth",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Might of Old Krosa",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": 1,
          "counter": "POISON",
          "count": 12
        }
      ]
    },
    {
      "name": "GainedInfectByEnchantment",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Phyrexian Obliterator",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Corrupted Conscience",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Enslave",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 9
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Corrupted Conscience",
          "target": "Phyrexian Obliterator"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Enslave",
          "target": "Phyrexian Obliterator"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Phyrexian Obliterator",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Corrupted Conscience",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Enslave",
          "count": 1
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Phyrexian Obliterator",
          "ability": "Infect",
          "expected": true
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": 1,
          "counter": "POISON",
          "count": 1
        }
      ]
    }
  ]
});
