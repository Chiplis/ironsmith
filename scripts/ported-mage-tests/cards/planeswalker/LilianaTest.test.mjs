import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/planeswalker/LilianaTest.java",
  "tests": [
    {
      "name": "testCreatureGainsZombieAsAdditionalType",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Binding Mummy",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Liliana, Death's Majesty",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Winged Shepherd",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Yoked Ox",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Liliana, Death's Majesty"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "-3:"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Winged Shepherd"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Yoked Ox"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Binding Mummy",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Liliana, Death's Majesty",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Winged Shepherd",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Yoked Ox",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Liliana, Death's Majesty",
          "counter": "LOYALTY",
          "count": 2
        },
        {
          "op": "unsupported",
          "source": "assertType(wShepherd, CardType.CREATURE, SubType.ZOMBIE)"
        },
        {
          "op": "unsupported",
          "source": "assertType(wShepherd, CardType.CREATURE, SubType.ANGEL)"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(yOx, true)"
        }
      ]
    },
    {
      "name": "testCastingCreaturesFromGraveTriggerDesecratedTomb",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Liliana, Untouched by Death",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Carrion Feeder",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Desecrated Tomb",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Liliana, Untouched by Death"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "-3:"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Carrion Feeder"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Liliana, Untouched by Death",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Liliana, Untouched by Death",
          "counter": "LOYALTY",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Carrion Feeder",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Bat Token",
          "count": 1
        }
      ]
    }
  ]
});
