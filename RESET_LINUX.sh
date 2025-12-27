#!/usr/bin/env sh
set -eu

echo "Resetting Vision node local data..."
echo "This will delete folders like: ./vision_data_7070/ (chain DB, peerbook, health DB)"
echo

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)

# Delete vision_data_* folders in the same directory as this script
for d in "$SCRIPT_DIR"/vision_data_*; do
  if [ -d "$d" ]; then
    echo "Deleting: $d"
    rm -rf -- "$d"
  fi
done

echo
echo "Done. You can now restart the node."
