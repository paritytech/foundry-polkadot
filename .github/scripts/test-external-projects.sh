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

  # Run tests
  if [ -n "$WORKING_DIRS" ]; then
    # Multiple directories
    IFS=',' read -ra DIRS <<< "$WORKING_DIRS"
    for dir in "${DIRS[@]}"; do
      echo "Testing in: $dir"
      cd "$dir"
      forge test --polkadot 2>&1 | tee -a "${GITHUB_WORKSPACE}/test-output-${PROJECT_NAME}.log"
      cd - > /dev/null
    done
  else
    # Single directory
    if [ -n "$WORKING_DIR" ]; then
      cd "$WORKING_DIR"
    fi
    forge test --polkadot 2>&1 | tee "${GITHUB_WORKSPACE}/test-output-${PROJECT_NAME}.log"
  fi

  cd "$GITHUB_WORKSPACE"
done
