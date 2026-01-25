#!/bin/bash
# run.sh
# Quick run script for Cuttlefish Docker container
#
# Usage:
#   ./run.sh              # Start Cuttlefish
#   ./run.sh shell        # Open shell in container
#   ./run.sh stop         # Stop Cuttlefish
#   ./run.sh logs         # View logs

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

IMAGE_NAME="${IMAGE_NAME:-cuttlefish-emulator:latest}"
CONTAINER_NAME="${CONTAINER_NAME:-cuttlefish}"

case "${1:-start}" in
    start|up)
        echo "Starting Cuttlefish..."
        docker-compose up -d
        echo ""
        echo "Cuttlefish starting. View logs with: ./run.sh logs"
        echo "Connect via ADB: adb connect localhost:6520"
        ;;

    stop|down)
        echo "Stopping Cuttlefish..."
        docker-compose down
        ;;

    restart)
        echo "Restarting Cuttlefish..."
        docker-compose restart
        ;;

    logs)
        docker-compose logs -f
        ;;

    shell|bash)
        docker-compose exec cuttlefish /bin/bash
        ;;

    adb)
        shift
        docker-compose exec cuttlefish docker-entrypoint.sh adb "$@"
        ;;

    status)
        docker-compose exec cuttlefish docker-entrypoint.sh status
        ;;

    ps)
        docker-compose ps
        ;;

    clean)
        echo "Stopping and removing all Cuttlefish data..."
        docker-compose down -v
        echo "Done. Run ./setup.sh to reinstall."
        ;;

    *)
        echo "Usage: $0 {start|stop|restart|logs|shell|adb|status|ps|clean}"
        echo ""
        echo "Commands:"
        echo "  start   - Start Cuttlefish container"
        echo "  stop    - Stop Cuttlefish container"
        echo "  restart - Restart Cuttlefish container"
        echo "  logs    - View container logs"
        echo "  shell   - Open bash shell in container"
        echo "  adb     - Run ADB command in container"
        echo "  status  - Show Cuttlefish status"
        echo "  ps      - Show container status"
        echo "  clean   - Remove all data and volumes"
        exit 1
        ;;
esac
