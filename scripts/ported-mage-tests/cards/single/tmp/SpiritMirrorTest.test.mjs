import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/tmp/SpiritMirrorTest.java",
  "tests": [
    {
      "name": "createsTokenBeginningOfUpkeepTest",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Spirit Mirror",
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
          "name": "Reflection Token",
          "count": 1
        }
      ]
    },
    {
      "name": "destroyCreatedTokenTest",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Spirit Mirror",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{0}: Destroy target Reflection",
          "target": "Reflection Token"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Reflection Token",
          "count": 0
        }
      ]
    },
    {
      "name": "onlyCreatesSingleTokenTest",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Spirit Mirror",
          "count": 1
        },
        {
          "op": "setStopAt",
          "turn": 20,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Reflection Token",
          "count": 1
        }
      ]
    },
    {
      "name": "destroyChangelingTest",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Chameleon Colossus",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Spirit Mirror",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "ability": "{0}: Destroy target Reflection",
          "target": "Chameleon Colossus"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Chameleon Colossus",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Chameleon Colossus",
          "count": 1
        }
      ]
    },
    {
      "name": "interactionWithUnnaturalSelectionTest",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Spirit Mirror",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Unnatural Selection",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Unnatural Selection",
          "ability": "Choose a creature type",
          "target": "Memnite"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Reflection"
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "source": "Spirit Mirror",
          "ability": "Destroy target Reflection",
          "target": "Memnite"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Chasm Skulker",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Unnatural Selection",
          "ability": "Choose a creature type",
          "target": "Chasm Skulker"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Reflection"
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "source": "Spirit Mirror",
          "ability": "Destroy target Reflection",
          "target": "Chasm Skulker"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Fabled Hero",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Unnatural Selection",
          "ability": "Choose a creature type",
          "target": "Fabled Hero"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Reflection"
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "source": "Spirit Mirror",
          "ability": "Destroy target Reflection",
          "target": "Fabled Hero"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Bronze Sable",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Unnatural Selection",
          "ability": "Choose a creature type",
          "target": "Bronze Sable"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Reflection"
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "source": "Spirit Mirror",
          "ability": "Destroy target Reflection",
          "target": "Bronze Sable"
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "UPKEEP"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Memnite",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Chasm Skulker",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Chasm Skulker",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Fabled Hero",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Fabled Hero",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Bronze Sable",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Bronze Sable",
          "count": 1
        }
      ]
    }
  ]
});
