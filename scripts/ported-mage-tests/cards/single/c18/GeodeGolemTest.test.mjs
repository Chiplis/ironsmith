import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/c18/GeodeGolemTest.java",
  "tests": [
    {
      "name": "test_Normal",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Geode Golem",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "COMMAND",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 10
        },
        {
          "op": "unsupported",
          "source": "addCustomEffect_TargetDamage(playerA, 2)"
        },
        {
          "op": "unsupported",
          "source": "checkCommandCardCount(\"before 1\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, \"Grizzly Bears\", 1)"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Geode Golem",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Grizzly Bears"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "COMBAT_DAMAGE",
          "player": null
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "COMBAT_DAMAGE",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "turn": 1,
          "phase": "COMBAT_DAMAGE",
          "player": 0,
          "seatIndependent": true,
          "name": "Forest",
          "tapped": true,
          "count": 0
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "target damage 2",
          "target": "Grizzly Bears"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Geode Golem",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Grizzly Bears"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "COMBAT_DAMAGE",
          "player": null
        },
        {
          "op": "assertPermanentCount",
          "turn": 3,
          "phase": "COMBAT_DAMAGE",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "turn": 3,
          "phase": "COMBAT_DAMAGE",
          "player": 0,
          "seatIndependent": true,
          "name": "Forest",
          "tapped": true,
          "count": 2
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "target damage 2",
          "target": "Grizzly Bears"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "attack",
          "turn": 5,
          "player": 0,
          "attacker": "Geode Golem",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Grizzly Bears"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "waitStackResolved",
          "turn": 5,
          "phase": "COMBAT_DAMAGE",
          "player": null
        },
        {
          "op": "assertPermanentCount",
          "turn": 5,
          "phase": "COMBAT_DAMAGE",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "turn": 5,
          "phase": "COMBAT_DAMAGE",
          "player": 0,
          "seatIndependent": true,
          "name": "Forest",
          "tapped": true,
          "count": 4
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        }
      ]
    },
    {
      "name": "test_MDF_SingleSide",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Geode Golem",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "COMMAND",
          "player": 0,
          "name": "Akoum Warrior",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 10
        },
        {
          "op": "unsupported",
          "source": "addCustomEffect_TargetDamage(playerA, 5)"
        },
        {
          "op": "unsupported",
          "source": "checkCommandCardCount(\"before 1\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, \"Akoum Warrior\", 1)"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Geode Golem",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Akoum Warrior"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "COMBAT_DAMAGE",
          "player": null
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "COMBAT_DAMAGE",
          "player": 0,
          "name": "Akoum Warrior",
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "turn": 1,
          "phase": "COMBAT_DAMAGE",
          "player": 0,
          "seatIndependent": true,
          "name": "Mountain",
          "tapped": true,
          "count": 0
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "target damage 5",
          "target": "Akoum Warrior"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Geode Golem",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Akoum Warrior"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "COMBAT_DAMAGE",
          "player": null
        },
        {
          "op": "assertPermanentCount",
          "turn": 3,
          "phase": "COMBAT_DAMAGE",
          "player": 0,
          "name": "Akoum Warrior",
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "turn": 3,
          "phase": "COMBAT_DAMAGE",
          "player": 0,
          "seatIndependent": true,
          "name": "Mountain",
          "tapped": true,
          "count": 2
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "target damage 5",
          "target": "Akoum Warrior"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "attack",
          "turn": 5,
          "player": 0,
          "attacker": "Geode Golem",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Akoum Warrior"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "waitStackResolved",
          "turn": 5,
          "phase": "COMBAT_DAMAGE",
          "player": null
        },
        {
          "op": "assertPermanentCount",
          "turn": 5,
          "phase": "COMBAT_DAMAGE",
          "player": 0,
          "name": "Akoum Warrior",
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "turn": 5,
          "phase": "COMBAT_DAMAGE",
          "player": 0,
          "seatIndependent": true,
          "name": "Mountain",
          "tapped": true,
          "count": 4
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Akoum Warrior",
          "count": 1
        }
      ]
    },
    {
      "name": "test_MDF_BothSides",
      "operations": [
        {
          "op": "setLife",
          "player": 1,
          "life": 25
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Geode Golem",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "COMMAND",
          "player": 0,
          "name": "Birgi, God of Storytelling",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 10
        },
        {
          "op": "unsupported",
          "source": "addCustomEffect_TargetDamage(playerA, 3)"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "target destroy",
          "count": 1,
          "custom": true,
          "fixtureSource": true,
          "typeLine": "",
          "oracleText": "{0}: Destroy target permanent."
        },
        {
          "op": "unsupported",
          "source": "checkCommandCardCount(\"before 1\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, \"Birgi, God of Storytelling\", 1)"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Geode Golem",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "options",
          "description": "Choose one",
          "choices": ["Cast Birgi, God of Storytelling"]
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "COMBAT_DAMAGE",
          "player": null
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "COMBAT_DAMAGE",
          "player": 0,
          "name": "Birgi, God of Storytelling",
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "turn": 1,
          "phase": "COMBAT_DAMAGE",
          "player": 0,
          "seatIndependent": true,
          "name": "Mountain",
          "tapped": true,
          "count": 0
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "target damage 3",
          "target": "Birgi, God of Storytelling"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Geode Golem",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "options",
          "description": "Choose one",
          "choices": ["Cast Birgi, God of Storytelling"]
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "COMBAT_DAMAGE",
          "player": null
        },
        {
          "op": "assertPermanentCount",
          "turn": 3,
          "phase": "COMBAT_DAMAGE",
          "player": 0,
          "name": "Birgi, God of Storytelling",
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "turn": 3,
          "phase": "COMBAT_DAMAGE",
          "player": 0,
          "seatIndependent": true,
          "name": "Mountain",
          "tapped": true,
          "count": 2
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "target damage 3",
          "target": "Birgi, God of Storytelling"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "attack",
          "turn": 5,
          "player": 0,
          "attacker": "Geode Golem",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "options",
          "description": "Choose one",
          "choices": ["Cast Harnfel, Horn of Bounty"]
        },
        {
          "op": "waitStackResolved",
          "turn": 5,
          "phase": "COMBAT_DAMAGE",
          "player": null
        },
        {
          "op": "assertPermanentCount",
          "turn": 5,
          "phase": "COMBAT_DAMAGE",
          "player": 0,
          "name": "Harnfel, Horn of Bounty",
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "turn": 5,
          "phase": "COMBAT_DAMAGE",
          "player": 0,
          "seatIndependent": true,
          "name": "Mountain",
          "tapped": true,
          "count": 4
        },
        {
          "op": "activateAbility",
          "turn": 5,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "target destroy",
          "target": "Harnfel, Horn of Bounty"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "attack",
          "turn": 7,
          "player": 0,
          "attacker": "Geode Golem",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "options",
          "description": "Choose one",
          "choices": ["Cast Harnfel, Horn of Bounty"]
        },
        {
          "op": "waitStackResolved",
          "turn": 7,
          "phase": "COMBAT_DAMAGE",
          "player": null
        },
        {
          "op": "assertPermanentCount",
          "turn": 7,
          "phase": "COMBAT_DAMAGE",
          "player": 0,
          "name": "Harnfel, Horn of Bounty",
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "turn": 7,
          "phase": "COMBAT_DAMAGE",
          "player": 0,
          "name": "Mountain",
          "tapped": true,
          "count": 6,
          "seatIndependent": true
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 7,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Harnfel, Horn of Bounty",
          "count": 1
        }
      ]
    }
  ]
});
