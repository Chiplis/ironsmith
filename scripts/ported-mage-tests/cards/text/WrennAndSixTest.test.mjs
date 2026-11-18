import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/text/WrennAndSixTest.java",
  "tests": [
    {
      "name": "testFirstLoyaltyAbilityRulesText",
      "operations": [
        {
          "op": "addCard",
          "player": 0,
          "zone": "BATTLEFIELD",
          "name": "Wrenn and Six"
        },
        {
          "op": "assertCompiledTextLine",
          "player": 0,
          "name": "Wrenn and Six",
          "index": 0,
          "text": "+1: Return up to one target land card from your graveyard to your hand."
        }
      ]
    }
  ]
});
