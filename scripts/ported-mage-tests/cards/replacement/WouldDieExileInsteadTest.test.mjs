import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/replacement/WouldDieExileInsteadTest.java",
  "tests": [
    {
      "name": "kalitasDamnationInteraction",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Kalitas, Traitor of Ghet",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Damnation",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Bronze Sable",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Wall of Roots",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Sigiled Starfish",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Damnation"
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
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Kalitas, Traitor of Ghet",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Damnation",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Bronze Sable",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Wall of Roots",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Sigiled Starfish",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 1,
          "name": 0
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Kalitas, Traitor of Ghet",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Zombie Token",
          "count": 3
        }
      ]
    },
    {
      "name": "magmaSpray_SoulScarMageEffect_ShouldExile",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Soul-Scar Mage",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Magma Spray",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Magma Spray",
          "target": "Grizzly Bears"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Soul-Scar Mage",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Magma Spray",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Grizzly Bears",
          "count": 0
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Soul-Scar Mage",
          "power": 2,
          "toughness": 3
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Grizzly Bears",
          "count": 0
        },
        {
          "op": "assertExileCount",
          "player": 1,
          "name": "Grizzly Bears",
          "count": 1
        }
      ]
    },
    {
      "name": "incendiaryFlow_SoulScarMageEffect_ShouldNotExile",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Soul-Scar Mage",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Incendiary Flow",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Hill Giant",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Incendiary Flow",
          "target": "Hill Giant"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Soul-Scar Mage",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Incendiary Flow",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Hill Giant",
          "count": 0
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Soul-Scar Mage",
          "power": 2,
          "toughness": 3
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Hill Giant",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 1,
          "name": "Hill Giant",
          "count": 0
        }
      ]
    },
    {
      "name": "miseryShadowReplacement",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Misery's Shadow",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Doom Blade",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Giant Spider",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Doom Blade",
          "target": "Giant Spider"
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
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Giant Spider",
          "count": 0
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Giant Spider",
          "count": 1
        }
      ]
    }
  ]
});
