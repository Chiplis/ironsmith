// Expected results from the pinned native CLI; reports/mse/plain-text-native-reference.json.
export const plainTextCases=[
  {
    "operation": "to_text",
    "input": "<b>bold</b> text",
    "expression": "to_text(\"<b>bold</b> text\")",
    "expected": "bold text"
  },
  {
    "operation": "to_text",
    "input": "before<sep-soft> hidden </sep-soft>after",
    "expression": "to_text(\"before<sep-soft> hidden </sep-soft>after\")",
    "expected": "beforeafter"
  },
  {
    "operation": "to_text",
    "input": "a<sep-soft>x<sep>nested</sep>y</sep-soft>b",
    "expression": "to_text(\"a<sep-soft>x<sep>nested</sep>y</sep-soft>b\")",
    "expected": "ab"
  },
  {
    "operation": "to_text",
    "input": "a<sep>keep</sep>b",
    "expression": "to_text(\"a<sep>keep</sep>b\")",
    "expected": "akeepb"
  },
  {
    "operation": "to_text",
    "input": "a<sep-soft>unfinished",
    "expression": "to_text(\"a<sep-soft>unfinished\")",
    "expected": ""
  },
  {
    "operation": "to_text",
    "input": "a<sep-soft>x</sep-soft>b<sep-soft>unfinished",
    "expression": "to_text(\"a<sep-soft>x</sep-soft>b<sep-soft>unfinished\")",
    "expected": "a"
  },
  {
    "operation": "to_text",
    "input": "a<unfinished",
    "expression": "to_text(\"a<unfinished\")",
    "expected": "a"
  },
  {
    "operation": "to_text",
    "input": "a > b",
    "expression": "to_text(\"a > b\")",
    "expected": "a > b"
  },
  {
    "operation": "to_text",
    "input": "ab",
    "expression": "to_text(\"ab\")",
    "expected": "a b"
  },
  {
    "operation": "to_text",
    "input": "a\u0001b",
    "expression": "to_text(\"a\\<b\")",
    "expected": "a<b"
  },
  {
    "operation": "remove_tags",
    "input": "<b>bold</b> text",
    "expression": "remove_tags(\"<b>bold</b> text\")",
    "expected": "bold text"
  },
  {
    "operation": "remove_tags",
    "input": "a<sep-soft>keep</sep-soft>b",
    "expression": "remove_tags(\"a<sep-soft>keep</sep-soft>b\")",
    "expected": "akeepb"
  },
  {
    "operation": "remove_tags",
    "input": "ab",
    "expression": "remove_tags(\"ab\")",
    "expected": "ab"
  },
  {
    "operation": "remove_tags",
    "input": "a\u0001b",
    "expression": "remove_tags(\"a\\<b\")",
    "expected": "a\u0001b"
  }
];
