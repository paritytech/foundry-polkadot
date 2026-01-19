#!/usr/bin/env bash
set -o pipefail

# Test external projects
# Usage: test-external-projects.sh

# Parse projects from PROJECTS environment variable
echo "$PROJECTS" | jq -c '.[]' | while read -r project; do
  PROJECT_NAME=$(echo "$project" | jq -r '.name')
  REPO=$(echo "$project" | jq -r '.repo')
  WORKING_DIR=$(echo "$project" | jq -r '.working_dir // ""')
  WORKING_DIRS=$(echo "$project" | jq -r '.working_dirs // ""')
  SETUP=$(echo "$project" | jq -r '.setup // ""')

  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
  echo "Testing: $PROJECT_NAME"
  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

  # Clone project
  git clone --depth 1 --recursive "https://github.com/$REPO" "test-projects/$PROJECT_NAME"
  cd "test-projects/$PROJECT_NAME"

  # Run setup if provided
  if [ -n "$SETUP" ]; then
    eval "$SETUP"
  fi

  OUTPUT_FILE="${GITHUB_WORKSPACE}/test-output-${PROJECT_NAME}.json"

  # Run tests with JSON output
  if [ -n "$WORKING_DIRS" ]; then
    # Multiple directories - merge JSON outputs
    IFS=',' read -ra DIRS <<< "$WORKING_DIRS"
    echo "{}" > "$OUTPUT_FILE"
    for dir in "${DIRS[@]}"; do
      echo "Testing in: $dir"
      cd "$dir"
      TEMP_JSON=$(mktemp)
      forge test --polkadot --json > "$TEMP_JSON" 2>&1 || true
      # Merge JSON outputs using jq
      if [ -s "$TEMP_JSON" ] && jq empty "$TEMP_JSON" 2>/dev/null; then
        jq -s '.[0] * .[1]' "$OUTPUT_FILE" "$TEMP_JSON" > "${OUTPUT_FILE}.tmp" && mv "${OUTPUT_FILE}.tmp" "$OUTPUT_FILE"
      fi
      rm -f "$TEMP_JSON"
      cd - > /dev/null
    done
  else
    # Single directory
    if [ -n "$WORKING_DIR" ]; then
      cd "$WORKING_DIR"
    fi
    forge test --polkadot --json > "$OUTPUT_FILE" 2>&1 || true
  fi

  cd "$GITHUB_WORKSPACE"
done
