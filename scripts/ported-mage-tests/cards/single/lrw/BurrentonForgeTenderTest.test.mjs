import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/lrw/BurrentonForgeTenderTest.java",
  "tests": [
    {
      "name": "testPreventDamageFromStack",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Burrenton Forge-Tender",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Soldier of the Pantheon",
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
          "player": 1,
          "name": "Lightning Bolt",
          "target": "Soldier of the Pantheon"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Sacrifice {this}: Prevent all damage a red source of your choice would deal this turn.",
          "target": "TestPlayer.NO_TARGET"
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
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Burrenton Forge-Tender",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Soldier of the Pantheon",
          "count": 1
        }
      ]
    },
    {
      "name": "testPreventDamageFromFlametongueKavu",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Burrenton Forge-Tender",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Soldier of the Pantheon",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Flametongue Kavu",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 4
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Flametongue Kavu"
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "Soldier of the Pantheon"
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Sacrifice {this}: Prevent all damage a red source of your choice would deal this turn.",
          "target": "TestPlayer.NO_TARGET"
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
          "player": 1,
          "name": "Flametongue Kavu",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Burrenton Forge-Tender",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Soldier of the Pantheon",
          "count": 1
        }
      ]
    },
    {
      "name": "testPreventDamageFromFlametongueKavuNotAfterCloudshift",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Burrenton Forge-Tender",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Soldier of the Pantheon",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Flametongue Kavu",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Cloudshift",
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
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Plains",
          "count": 5
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Flametongue Kavu"
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "Soldier of the Pantheon"
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Sacrifice {this}: Prevent all damage a red source of your choice would deal this turn.",
          "target": "TestPlayer.NO_TARGET"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Cloudshift",
          "target": "Flametongue Kavu"
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "Soldier of the Pantheon"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Flametongue Kavu",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Cloudshift",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Burrenton Forge-Tender",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Soldier of the Pantheon",
          "count": 1
        }
      ]
    },
    {
      "name": "testPreventDamageFromToken",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Burrenton Forge-Tender",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Soldier of the Pantheon",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mogg Fanatic",
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
          "player": 0,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Orzhov Charm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Rite of Replication",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 4
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Rite of Replication",
          "target": "Mogg Fanatic"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": false
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Orzhov Charm",
          "target": "Mogg Fanatic"
        },
        {
          "op": "setModeChoice",
          "player": 0,
          "value": "1"
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "END_COMBAT",
          "player": 0,
          "ability": "Sacrifice {this}: Prevent all damage a red source of your choice would deal this turn."
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "ability": "Sacrifice {this}: It deals 1 damage to ",
          "target": "Soldier of the Pantheon"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Mogg Fanatic",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Orzhov Charm",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Mogg Fanatic",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Rite of Replication",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Burrenton Forge-Tender",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Soldier of the Pantheon",
          "count": 1
        }
      ]
    }
  ]
});
