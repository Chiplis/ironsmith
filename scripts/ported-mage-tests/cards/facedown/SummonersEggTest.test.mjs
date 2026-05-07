import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/facedown/SummonersEggTest.java",
  "tests": [
    {
      "name": "testSummonersEggImprint",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Summoner's Egg",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Maritime Guard",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Goblin Roughrider",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 4
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Summoner's Egg"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Goblin Roughrider"
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
          "op": "assertHandCount",
          "player": 0,
          "count": 0,
          "name": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Maritime Guard",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Goblin Roughrider",
          "count": 0
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Goblin Roughrider",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "for (Card card : currentGame.getExile().getAllCards(currentGame)) { if (card.getName().equals(\"Goblin Roughrider\")) { Assert.assertTrue(\"Exiled card is not face down\", card.isFaceDown(currentGame)); } }"
        }
      ]
    },
    {
      "name": "testSummonersEggDies",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Summoner's Egg",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Maritime Guard",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Goblin Roughrider",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Char",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 3
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Summoner's Egg"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Goblin Roughrider"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Char",
          "target": "Summoner's Egg"
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
          "op": "assertHandCount",
          "player": 0,
          "count": 0,
          "name": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Maritime Guard",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Goblin Roughrider",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Summoner's Egg",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Goblin Roughrider",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Goblin Roughrider",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "for (Permanent p : currentGame.getBattlefield().getAllActivePermanents()) { if (p.getName().equals(\"Goblin Roughrider\")) { Assert.assertTrue(\"Permanent is not face up\", !p.isFaceDown(currentGame)); } }"
        }
      ]
    },
    {
      "name": "testSummonersEggDies2",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Summoner's Egg",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Forest",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Char",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 3
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Summoner's Egg"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Forest"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Char",
          "target": "Summoner's Egg"
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
          "op": "assertHandCount",
          "player": 0,
          "count": 0,
          "name": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Summoner's Egg",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "for (Card card : currentGame.getExile().getAllCards(currentGame)) { if (card.getName().equals(\"Forest\")) { Assert.assertTrue(\"Exiled card is not face up\", !card.isFaceDown(currentGame)); } }"
        }
      ]
    }
  ]
});
