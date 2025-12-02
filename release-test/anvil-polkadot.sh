#!/bin/sh

# docker run --rm -it -v $PWD/test:/test -w /test foundry sh

REPOSITORY="test"

> anvil-polkadot.txt

clear

rm -rf $REPOSITORY && mkdir $REPOSITORY

docker_run() {
  echo "\033[0;32m> $@\033[0m"

  echo "> $@" >> anvil-polkadot.txt

  cache=$(mktemp)

  # Note: running anvil requires passing the command string correctly if it contains shell operators
  docker run --rm -v "$PWD/$REPOSITORY":/"$REPOSITORY" -w /"$REPOSITORY" foundry "$@" > "$cache" 2>&1

  status=$?

  cat "$cache" | tee -a anvil-polkadot.txt

  rm -f "$cache"

  if [ $status -ne 0 ]; then
    echo "\033[0;31mERROR: Command failed with exit status $status: $@\033[0m"
    exit 1
  fi
}

# 1. Basic Binary Checks
# ----------------------
docker_run anvil-polkadot --version
docker_run anvil-polkadot --help

# 2. Liveness Tests
# ----------------------
# Anvil is a server, so we must start it in the background, wait, and then check if it's still alive.
# We use 'sh -c' to wrap the backgrounding and killing logic inside the container.
# If anvil crashes on startup, 'kill $PID' will fail, causing the test to fail.

echo "\033[0;33mTesting Basic Startup...\033[0m"
docker_run sh -c "anvil-polkadot & PID=\$!; sleep 3; kill \$PID"

# 3. Mining Test
# ----------------------
# Test Block Time configuration.
# We use 1 second block time so we can see blocks produced within the 5 second sleep window.
echo "\033[0;33mTesting Block Time (--block-time 1)...\033[0m"
docker_run sh -c "anvil-polkadot --block-time 1 & PID=\$!; sleep 5; kill \$PID"

# 4. Account Test
# ----------------------
# Test Account Generation configuration (Sanity check for CLI parsing)
echo "\033[0;33mTesting Account Configuration (--accounts)...\033[0m"
docker_run sh -c "anvil-polkadot --accounts 5 --balance 1000 & PID=\$!; sleep 3; kill \$PID"

echo "\033[0;32mAll anvil-polkadot tests passed!\033[0m"
