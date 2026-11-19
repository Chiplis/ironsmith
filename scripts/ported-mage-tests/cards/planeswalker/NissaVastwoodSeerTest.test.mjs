import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/planeswalker/NissaVastwoodSeerTest.java",
  "tests": [
    {
      "name": "NissaVastwoodSeerAnimationTest",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 5
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
          "zone": "HAND",
          "player": 0,
          "name": "Nissa, Vastwood Seer",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Nissa, Vastwood Seer",
          "resolve": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Forest"
        },
        {
          "op": "assertHandCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "playLand",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Forest"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "+1: Reveal",
          "source": "Nissa, Sage Animist"
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "+1: Reveal",
          "source": "Nissa, Sage Animist"
        },
        {
          "op": "activateAbility",
          "turn": 5,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "+1: Reveal",
          "source": "Nissa, Sage Animist"
        },
        {
          "op": "activateAbility",
          "turn": 7,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "+1: Reveal",
          "source": "Nissa, Sage Animist"
        },
        {
          "op": "activateAbility",
          "turn": 9,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "-7: Untap up to six target",
          "source": "Nissa, Sage Animist"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Forest^Forest^Forest^Forest^Forest^Forest"
        },
        {
          "op": "setStopAt",
          "turn": 9,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Nissa, Vastwood Seer",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Nissa, Sage Animist",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Nissa, Vastwood Seer",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Swamp",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Forest\", CardType.CREATURE, SubType.ELEMENTAL)"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Forest",
          "count": 6
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Forest",
          "power": 6,
          "toughness": 6,
          "scope": "All"
        }
      ]
    }
  ]
});
