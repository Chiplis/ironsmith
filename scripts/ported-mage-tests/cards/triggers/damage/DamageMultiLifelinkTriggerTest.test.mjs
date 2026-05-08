import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/triggers/damage/DamageMultiLifelinkTriggerTest.java",
  "tests": [
    {
      "name": "testCreatureDamageTargetAndYou",
      "operations": [
        {
          "op": "unsupported",
          "source": "setupBattlefield()"
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Forge Devil",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Forge Devil"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Wishcoin Crab"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "life": 21
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerB, wishcoin, 1)"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Ajani's Pridemate",
          "counter": "P1P1",
          "count": 1
        }
      ],
      "skip": "upstream @Ignore"
    },
    {
      "name": "testCreatureDamageTargetAndSelf",
      "operations": [
        {
          "op": "unsupported",
          "source": "setupBattlefield()"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Reckless Embermage",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{1}{R}: "
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Wishcoin Crab"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "life": 22
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerB, wishcoin, 1)"
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerB, embermage, 1)"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Ajani's Pridemate",
          "counter": "P1P1",
          "count": 1
        }
      ],
      "skip": "upstream @Ignore"
    },
    {
      "name": "testSpellDamageTargetAndTarget",
      "operations": [
        {
          "op": "unsupported",
          "source": "setupBattlefield()"
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Arc Trail",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Arc Trail"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Wishcoin Crab"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Kraken Hatchling"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "life": 23
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerB, wishcoin, 2)"
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerA, hatchling, 1)"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Ajani's Pridemate",
          "counter": "P1P1",
          "count": 1
        }
      ],
      "skip": "upstream @Ignore"
    },
    {
      "name": "testSpellDamageThreeTargets",
      "operations": [
        {
          "op": "unsupported",
          "source": "setupBattlefield()"
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Cone of Flame",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Cone of Flame"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Wishcoin Crab"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Kraken Hatchling"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Firesong and Sunspeaker"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "life": 26
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerB, wishcoin, 3)"
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerA, hatchling, 2)"
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerA, firesong, 1)"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Ajani's Pridemate",
          "counter": "P1P1",
          "count": 1
        }
      ],
      "skip": "upstream @Ignore"
    },
    {
      "name": "testSpellDamageTargetAndYou",
      "operations": [
        {
          "op": "unsupported",
          "source": "setupBattlefield()"
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Char",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Char"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Wishcoin Crab"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerB, wishcoin, 4)"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Ajani's Pridemate",
          "counter": "P1P1",
          "count": 1
        }
      ],
      "skip": "upstream @Ignore"
    },
    {
      "name": "testSpellDamageTargetAndController",
      "operations": [
        {
          "op": "unsupported",
          "source": "setupBattlefield()"
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Chandra's Outrage",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Chandra's Outrage"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Wishcoin Crab"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "life": 26
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 18
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerB, wishcoin, 4)"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Ajani's Pridemate",
          "counter": "P1P1",
          "count": 1
        }
      ],
      "skip": "upstream @Ignore"
    },
    {
      "name": "testSpellDamagePlayerAndControlled",
      "operations": [
        {
          "op": "unsupported",
          "source": "setupBattlefield()"
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Chandra's Fury",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Chandra's Fury"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "life": 25
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 16
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerB, wishcoin, 1)"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Ajani's Pridemate",
          "counter": "P1P1",
          "count": 1
        }
      ],
      "skip": "upstream @Ignore"
    }
  ]
});
