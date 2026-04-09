#!/usr/bin/env sh
set -eu

# Usage:
#   ./scripts/run-profile.sh edge
#   ./scripts/run-profile.sh gateway
#   ./scripts/run-profile.sh server
#
# Optional second argument:
#   up (default), down, restart, logs

PROFILE="${1:-gateway}"
ACTION="${2:-up}"
ENV_FILE="deploy/profiles/${PROFILE}.env"

if [ ! -f "$ENV_FILE" ]; then
    echo "Unknown profile '$PROFILE'. Expected: edge | gateway | server" >&2
    exit 1
fi

case "$ACTION" in
    up)
        docker compose --env-file "$ENV_FILE" up -d
        ;;
    down)
        docker compose --env-file "$ENV_FILE" down
        ;;
    restart)
        docker compose --env-file "$ENV_FILE" down
        docker compose --env-file "$ENV_FILE" up -d
        ;;
    logs)
        docker compose --env-file "$ENV_FILE" logs -f maverick
        ;;
    *)
        echo "Unknown action '$ACTION'. Expected: up | down | restart | logs" >&2
        exit 1
        ;;
esac
