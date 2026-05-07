import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/mkm/TenthDistrictHeroTest.java",
  "tests": [
    {
      "name": "testFirstAblityOnly",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Tenth District Hero",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Hill Giant",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{1}"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Hill Giant"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Tenth District Hero",
          "power": 4,
          "toughness": 4
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(hero, SubType.HUMAN)"
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(hero, SubType.DETECTIVE)"
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Tenth District Hero",
          "ability": "Vigilance",
          "expected": true
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Mileva, the Stalwart",
          "count": 0
        }
      ]
    },
    {
      "name": "testSecondAblityOnly",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Tenth District Hero",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Hill Giant",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{2}"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Hill Giant"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Tenth District Hero",
          "power": 2,
          "toughness": 3
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(hero, SubType.HUMAN)"
        },
        {
          "op": "unsupported",
          "source": "assertNotSubtype(hero, SubType.DETECTIVE)"
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Tenth District Hero",
          "ability": "Vigilance",
          "expected": false
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Tenth District Hero",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Mileva, the Stalwart",
          "count": 0
        }
      ]
    },
    {
      "name": "testBothAbilities",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Tenth District Hero",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Hill Giant",
          "count": 2
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{1}"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Hill Giant"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "TestPlayer.CHOICE_SKIP"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{2}"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Hill Giant"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Mileva, the Stalwart",
          "power": 5,
          "toughness": 5
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(mileva, SubType.HUMAN)"
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(mileva, SubType.DETECTIVE)"
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Mileva, the Stalwart",
          "ability": "Vigilance",
          "expected": true
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Mileva, the Stalwart",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Tenth District Hero",
          "count": 0
        }
      ]
    },
    {
      "name": "testBothAbilitiesReversed",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Tenth District Hero",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Hill Giant",
          "count": 2
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{2}"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Hill Giant"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "TestPlayer.CHOICE_SKIP"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{1}"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Hill Giant"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Tenth District Hero",
          "power": 4,
          "toughness": 4
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(hero, SubType.HUMAN)"
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(hero, SubType.DETECTIVE)"
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Tenth District Hero",
          "ability": "Vigilance",
          "expected": true
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Mileva, the Stalwart",
          "count": 0
        }
      ]
    }
  ]
});
