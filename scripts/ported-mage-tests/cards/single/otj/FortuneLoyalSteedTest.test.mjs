import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/otj/FortuneLoyalSteedTest.java",
  "tests": [
    {
      "name": "test_Saddling",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Fortune, Loyal Steed",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Lone Missionary",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Taiga",
          "count": 2
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Lone Missionary"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Saddle"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Fortune, Loyal Steed",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Lone Missionary"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "When {this} enters, you gain 4 life"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Taiga"
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
          "player": 0,
          "life": 24
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0,
          "name": 0
        },
        {
          "op": "unsupported",
          "source": "assertTapped(fortune, false)"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Lone Missionary\", false)"
        }
      ]
    },
    {
      "name": "test_Saddling_FortuneDies",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Fortune, Loyal Steed",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Lone Missionary",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Ankle Biter",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Lone Missionary"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Saddle"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Fortune, Loyal Steed",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Ankle Biter",
          "attacker": "Fortune, Loyal Steed"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Lone Missionary"
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
          "player": 0,
          "life": 24
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Fortune, Loyal Steed",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Lone Missionary\", false)"
        }
      ]
    },
    {
      "name": "test_Saddling_FortuneBlinks",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Fortune, Loyal Steed",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Lone Missionary",
          "count": 1
        },
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
          "name": "Ephemerate",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Taiga",
          "count": 2
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Lone Missionary"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Saddle"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Fortune, Loyal Steed",
          "defender": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS",
          "player": 0,
          "name": "Ephemerate",
          "target": "Fortune, Loyal Steed"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Taiga"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Lone Missionary"
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
          "player": 0,
          "life": 24
        },
        {
          "op": "unsupported",
          "source": "assertTapped(fortune, false)"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Lone Missionary\", false)"
        }
      ]
    },
    {
      "name": "test_Saddling_FortuneBlinksBefore",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Fortune, Loyal Steed",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Fervor",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Lone Missionary",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Ephemerate",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Taiga",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "name": "Ephemerate",
          "target": "Fortune, Loyal Steed"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Taiga"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Lone Missionary"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Saddle"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Fortune, Loyal Steed",
          "defender": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS",
          "player": 0,
          "name": "Ephemerate",
          "target": "Fortune, Loyal Steed"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Taiga"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Lone Missionary"
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
          "player": 0,
          "life": 24
        },
        {
          "op": "unsupported",
          "source": "assertTapped(fortune, false)"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Lone Missionary\", false)"
        }
      ]
    },
    {
      "name": "test_Saddling_FortuneBlinksAfterSaddlingBeforeCombat",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Fortune, Loyal Steed",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Fervor",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Lone Missionary",
          "count": 1
        },
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
          "name": "Ephemerate",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Taiga",
          "count": 2
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Lone Missionary"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Saddle"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Ephemerate",
          "target": "Fortune, Loyal Steed"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Taiga"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Fortune, Loyal Steed",
          "defender": 1
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
          "player": 0,
          "life": 20
        },
        {
          "op": "unsupported",
          "source": "assertTapped(fortune, true)"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Lone Missionary\", true)"
        }
      ]
    },
    {
      "name": "test_Saddling_FortuneBlinksInResponseOfSaddling",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Fortune, Loyal Steed",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Fervor",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Lone Missionary",
          "count": 1
        },
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
          "name": "Ephemerate",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Taiga",
          "count": 2
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Lone Missionary"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Saddle"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Ephemerate",
          "target": "Fortune, Loyal Steed"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Taiga"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Fortune, Loyal Steed",
          "defender": 1
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
          "player": 0,
          "life": 20
        },
        {
          "op": "unsupported",
          "source": "assertTapped(fortune, true)"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Lone Missionary\", true)"
        }
      ]
    },
    {
      "name": "test_Saddling_FortuneBlinksTwice",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Fortune, Loyal Steed",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Lone Missionary",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Ephemerate",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Taiga",
          "count": 2
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Lone Missionary"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Saddle"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Fortune, Loyal Steed",
          "defender": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS",
          "player": 0,
          "name": "Ephemerate",
          "target": "Fortune, Loyal Steed"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Taiga"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS",
          "player": 0,
          "name": "Ephemerate",
          "target": "Fortune, Loyal Steed"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Taiga"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Lone Missionary"
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
          "player": 0,
          "life": 24
        },
        {
          "op": "unsupported",
          "source": "assertTapped(fortune, false)"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Lone Missionary\", false)"
        }
      ]
    }
  ]
});
