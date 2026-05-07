import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/soi/PrizedAmalgamTest.java",
  "tests": [
    {
      "name": "testReanimation",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Reanimate",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Bronze Sable",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Prized Amalgam",
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
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Reanimate",
          "target": "Bronze Sable"
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
          "op": "assertLife",
          "player": 0,
          "life": 18
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Reanimate",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Bronze Sable",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Prized Amalgam",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Prized Amalgam\", true)"
        }
      ]
    },
    {
      "name": "testGravecrawlerCastFromGrave",
      "operations": [
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Gravecrawler",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Prized Amalgam",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gnawing Zombie",
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
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Gravecrawler"
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
          "name": "Gravecrawler",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Prized Amalgam",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Prized Amalgam\", true)"
        }
      ]
    },
    {
      "name": "testOpponentReturnsCreatureFromGrave",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Reanimate",
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
          "name": "Mountain",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 1,
          "name": "Prized Amalgam",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Reanimate"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Hill Giant"
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
          "op": "assertLife",
          "player": 0,
          "life": 16
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Hill Giant",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Prized Amalgam",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Prized Amalgam",
          "count": 1
        }
      ]
    },
    {
      "name": "testOpponentReturnsCreatureFromYourGrave",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Necromantic Summons",
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
          "zone": "GRAVEYARD",
          "player": 1,
          "name": "Merfolk Looter",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 1,
          "name": "Prized Amalgam",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Necromantic Summons",
          "target": "Merfolk Looter"
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
          "name": "Merfolk Looter",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Prized Amalgam",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Prized Amalgam",
          "count": 0
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Prized Amalgam\", true)"
        }
      ]
    }
  ]
});
