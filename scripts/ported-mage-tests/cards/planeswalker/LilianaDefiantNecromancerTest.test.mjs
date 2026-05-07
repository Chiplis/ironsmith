import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/planeswalker/LilianaDefiantNecromancerTest.java",
  "tests": [
    {
      "name": "testMinusAbilityShouldNotReturnHigherCmcCreature",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Hill Giant",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Liliana, Heretical Healer",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Liliana, Heretical Healer"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 1,
          "name": "Lightning Bolt",
          "target": "Hill Giant"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "-X:",
          "target": "Hill Giant"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=1"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "unsupported",
          "source": "try { execute(); Assert.fail(\"must throw exception on execute\"); } catch (Throwable e) { if (!e.getMessage().contains(\"-X:$target=Hill Giant\")) { Assert.fail(\"Should have thrown error about cannot attack, but got:\\n\" + e.getMessage()); } } assertGraveyardCount(playerB, \"Lightning Bolt\", 1)"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Liliana, Heretical Healer",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Zombie Token",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Liliana, Defiant Necromancer",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Liliana, Defiant Necromancer",
          "counter": "LOYALTY",
          "count": 3
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Hill Giant",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Hill Giant",
          "count": 1
        }
      ]
    },
    {
      "name": "testMinusAbilityShouldNotReturnLegendaryCreature",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Alesha, Who Smiles at Death",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Liliana, Heretical Healer",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Liliana, Heretical Healer"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 1,
          "name": "Lightning Bolt"
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "Alesha, Who Smiles at Death"
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "label": "-X:",
          "expected": false
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
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Liliana, Heretical Healer",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Zombie Token",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Alesha, Who Smiles at Death",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Alesha, Who Smiles at Death",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Liliana, Defiant Necromancer",
          "counter": "LOYALTY",
          "count": 3
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Liliana, Defiant Necromancer",
          "count": 1
        }
      ]
    },
    {
      "name": "testMinusAbilityShouldReturnNonLegendaryCreature",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Bronze Sable",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Liliana, Heretical Healer",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Liliana, Heretical Healer"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 1,
          "name": "Lightning Bolt"
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "Bronze Sable"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "-X:"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=2"
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
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Liliana, Heretical Healer",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Zombie Token",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Bronze Sable",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Bronze Sable",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Liliana, Defiant Necromancer",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Liliana, Defiant Necromancer",
          "counter": "LOYALTY",
          "count": 1
        }
      ]
    }
  ]
});
