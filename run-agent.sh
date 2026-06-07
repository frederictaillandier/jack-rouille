#!/usr/bin/env bash
#
# Wrapper for scheduled (cron) runs of review-agent. Sets a predictable PATH
# (cron has a minimal one), builds, and runs the agent from the project dir.
#
set -euo pipefail

# Tools live in the user's local bins, which cron's default PATH omits.
export PATH="$HOME/.local/bin:$HOME/.cargo/bin:/usr/local/bin:/usr/bin:/bin"

# Run as the bot (jack-rouille) when a token is present: gh + git push both use
# it over HTTPS, so PRs are bot-authored and you review them with your account.
# Absent the file, the agent falls back to your ambient gh/SSH identity.
TOKEN_FILE="$HOME/.config/jack-rouille.token"
if [ -f "$TOKEN_FILE" ]; then
    GH_TOKEN="$(tr -d '[:space:]' < "$TOKEN_FILE")"
    export GH_TOKEN
fi

cd "$(dirname "$(readlink -f "$0")")"

# Pace runs against the weekly Claude budget. The OAuth usage endpoint reports
# *utilization* (percent of the rolling 7-day allowance consumed) and when the
# window resets. We compare how much of the week is left against how many tokens
# are left, and only run when tokens left is more than double the time left —
# i.e. there's surplus budget to burn before the reset. Any failure (no creds,
# network, bad response) fails open: we run anyway.
usage_allows_run() {
    local creds="$HOME/.claude/.credentials.json"
    [ -f "$creds" ] || { echo "usage: no credentials at $creds — running anyway"; return 0; }

    local token
    token="$(jq -r '.claudeAiOauth.accessToken // empty' "$creds")"
    [ -n "$token" ] || { echo "usage: no access token in $creds — running anyway"; return 0; }

    local resp
    if ! resp="$(curl -sS -X GET https://api.anthropic.com/api/oauth/usage \
        -H "Authorization: Bearer $token" \
        -H "anthropic-beta: oauth-2025-04-20")"; then
        echo "usage: request failed — running anyway"; return 0
    fi
    jq -e . >/dev/null 2>&1 <<<"$resp" || { echo "usage: unexpected response — running anyway: $resp"; return 0; }

    local util resets
    util="$(jq -r '.seven_day.utilization // 0' <<<"$resp")"
    resets="$(jq -r '.seven_day.resets_at // empty' <<<"$resp")"
    [ -n "$resets" ] || { echo "usage: no reset time — running anyway"; return 0; }

    local resets_epoch now
    resets_epoch="$(date -d "$resets" +%s 2>/dev/null)" || { echo "usage: bad reset time '$resets' — running anyway"; return 0; }
    now="$(date +%s)"

    # time_pct = percent of the 7-day window still left; token_pct = percent of
    # the allowance still left. Run only when token_pct > 2 * time_pct.
    awk -v util="$util" -v left="$((resets_epoch - now))" 'BEGIN {
        week = 7*24*3600;
        time_pct  = left / week * 100;
        token_pct = 100 - util;
        run = (time_pct * 2 < token_pct);
        printf "usage: tokens left=%.0f%%, week left=%.0f%% -> %s\n", token_pct, time_pct, (run ? "RUN" : "SKIP");
        exit (run ? 0 : 1);
    }'
}

if ! usage_allows_run; then
    echo "===== $(date '+%Y-%m-%d %H:%M:%S %z') — skipping run: conserving weekly Claude budget ====="
    exit 0
fi

echo "===== $(date '+%Y-%m-%d %H:%M:%S %z') — review-agent run starting ====="
cargo build --release --quiet
./target/release/review-agent
echo "===== $(date '+%Y-%m-%d %H:%M:%S %z') — review-agent run finished ====="
