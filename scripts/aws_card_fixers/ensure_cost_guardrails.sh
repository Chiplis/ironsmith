#!/usr/bin/env bash
set -euo pipefail

AWS_REGION="${AWS_REGION:-us-east-1}"
FLEET_REGION="${FLEET_REGION:-us-east-2}"
BUDGET_ALERT_EMAIL="${BUDGET_ALERT_EMAIL:-}"
BUDGET_MONTHLY_LIMIT_USD="${BUDGET_MONTHLY_LIMIT_USD:-50}"
BUDGET_DAILY_LIMIT_USD="${BUDGET_DAILY_LIMIT_USD:-10}"
ANOMALY_ALERT_THRESHOLD_USD="${ANOMALY_ALERT_THRESHOLD_USD:-10}"
CREATE_ANOMALY_DETECTION="${CREATE_ANOMALY_DETECTION:-1}"

AWS=(aws)
if [[ -n "${AWS_PROFILE:-}" ]]; then
  AWS+=(--profile "$AWS_PROFILE")
fi
AWS+=(--region "$AWS_REGION")

usage() {
  cat <<EOF
Usage:
  BUDGET_ALERT_EMAIL=user@example.com scripts/aws_card_fixers/ensure_cost_guardrails.sh

Optional env:
  AWS_PROFILE=
  AWS_REGION=us-east-1
  FLEET_REGION=us-east-2
  BUDGET_MONTHLY_LIMIT_USD=50
  BUDGET_DAILY_LIMIT_USD=10
  ANOMALY_ALERT_THRESHOLD_USD=10
  CREATE_ANOMALY_DETECTION=1
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

ACCOUNT_ID="$("${AWS[@]}" sts get-caller-identity --query Account --output text)"
if [[ -z "$BUDGET_ALERT_EMAIL" ]]; then
  caller_arn="$("${AWS[@]}" sts get-caller-identity --query Arn --output text)"
  if [[ "$caller_arn" == *"@"* ]]; then
    BUDGET_ALERT_EMAIL="${caller_arn##*/}"
  fi
fi
if [[ -z "$BUDGET_ALERT_EMAIL" ]]; then
  echo "BUDGET_ALERT_EMAIL is required when it cannot be inferred from the caller ARN." >&2
  exit 2
fi

TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

write_budget() {
  local name="$1"
  local amount="$2"
  local time_unit="$3"
  local budget_file="$TMPDIR/${name}.json"
  local notifications_file="$TMPDIR/${name}-notifications.json"

  python3 - "$budget_file" "$notifications_file" "$name" "$amount" "$time_unit" "$BUDGET_ALERT_EMAIL" <<'PY'
import json
import pathlib
import sys

budget_path = pathlib.Path(sys.argv[1])
notifications_path = pathlib.Path(sys.argv[2])
name, amount, time_unit, email = sys.argv[3:]
services = [
    "Amazon Elastic Compute Cloud - Compute",
    "EC2 - Other",
    "Amazon Simple Storage Service",
]

budget_path.write_text(
    json.dumps(
        {
            "BudgetName": name,
            "BudgetLimit": {"Amount": amount, "Unit": "USD"},
            "CostFilters": {"Service": services},
            "BudgetType": "COST",
            "TimeUnit": time_unit,
        },
        indent=2,
        sort_keys=True,
    )
    + "\n",
    encoding="utf-8",
)

notifications = [
    {
        "Notification": {
            "NotificationType": "ACTUAL",
            "ComparisonOperator": "GREATER_THAN",
            "Threshold": threshold,
            "ThresholdType": "PERCENTAGE",
        },
        "Subscribers": [{"SubscriptionType": "EMAIL", "Address": email}],
    }
    for threshold in (80, 100)
]
if time_unit != "DAILY":
    notifications.append(
        {
            "Notification": {
                "NotificationType": "FORECASTED",
                "ComparisonOperator": "GREATER_THAN",
                "Threshold": 100,
                "ThresholdType": "PERCENTAGE",
            },
            "Subscribers": [{"SubscriptionType": "EMAIL", "Address": email}],
        }
    )
notifications_path.write_text(json.dumps(notifications, indent=2) + "\n", encoding="utf-8")
PY

  if "${AWS[@]}" budgets describe-budget --account-id "$ACCOUNT_ID" --budget-name "$name" >/dev/null 2>&1; then
    "${AWS[@]}" budgets update-budget \
      --account-id "$ACCOUNT_ID" \
      --new-budget "file://${budget_file}" >/dev/null
    echo "Updated budget ${name}."
  else
    "${AWS[@]}" budgets create-budget \
      --account-id "$ACCOUNT_ID" \
      --budget "file://${budget_file}" \
      --notifications-with-subscribers "file://${notifications_file}" >/dev/null
    echo "Created budget ${name}."
  fi
}

write_budget IronsmithFleetMonthlyCost "$BUDGET_MONTHLY_LIMIT_USD" MONTHLY
write_budget IronsmithFleetDailyCost "$BUDGET_DAILY_LIMIT_USD" DAILY

if [[ "$CREATE_ANOMALY_DETECTION" != "1" ]]; then
  exit 0
fi

monitor_name="IronsmithFleetServices"
subscription_name="IronsmithFleetAnomalyAlerts"
monitor_arn="$("${AWS[@]}" ce get-anomaly-monitors \
  --query "AnomalyMonitors[?MonitorName=='${monitor_name}'].MonitorArn | [0]" \
  --output text 2>/dev/null || true)"
if [[ -z "$monitor_arn" || "$monitor_arn" == "None" ]]; then
  monitor_file="$TMPDIR/anomaly-monitor.json"
  python3 - "$monitor_file" "$monitor_name" <<'PY'
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
name = sys.argv[2]
path.write_text(
    json.dumps(
        {
            "MonitorName": name,
            "MonitorType": "DIMENSIONAL",
            "MonitorDimension": "SERVICE",
        },
        indent=2,
        sort_keys=True,
    )
    + "\n",
    encoding="utf-8",
)
PY
  monitor_arn="$("${AWS[@]}" ce create-anomaly-monitor \
    --anomaly-monitor "file://${monitor_file}" \
    --query MonitorArn \
    --output text)"
  echo "Created cost anomaly monitor ${monitor_name}."
else
  echo "Cost anomaly monitor ${monitor_name} already exists."
fi

subscription_arn="$("${AWS[@]}" ce get-anomaly-subscriptions \
  --query "AnomalySubscriptions[?SubscriptionName=='${subscription_name}'].SubscriptionArn | [0]" \
  --output text 2>/dev/null || true)"
if [[ -z "$subscription_arn" || "$subscription_arn" == "None" ]]; then
  subscription_file="$TMPDIR/anomaly-subscription.json"
  python3 - "$subscription_file" "$subscription_name" "$monitor_arn" "$BUDGET_ALERT_EMAIL" "$ANOMALY_ALERT_THRESHOLD_USD" <<'PY'
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
name, monitor_arn, email, threshold = sys.argv[2:]
path.write_text(
    json.dumps(
        {
            "SubscriptionName": name,
            "MonitorArnList": [monitor_arn],
            "Frequency": "DAILY",
            "Subscribers": [{"Type": "EMAIL", "Address": email}],
            "Threshold": float(threshold),
        },
        indent=2,
        sort_keys=True,
    )
    + "\n",
    encoding="utf-8",
)
PY
  "${AWS[@]}" ce create-anomaly-subscription \
    --anomaly-subscription "file://${subscription_file}" >/dev/null
  echo "Created cost anomaly subscription ${subscription_name}."
else
  echo "Cost anomaly subscription ${subscription_name} already exists."
fi
