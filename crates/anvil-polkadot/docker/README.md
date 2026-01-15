## anvil-polkadot Docker image

This folder contains a Dockerfile for running an `anvil-polkadot` node in Docker.
By default it builds from your **current workspace source**.

### Quick start (local workspace)

From the repository root:

```sh
docker build -f crates/anvil-polkadot/docker/Dockerfile -t anvil-polkadot .
docker run --rm -p 8545:8545 anvil-polkadot
```

To pass extra CLI flags to `anvil-polkadot`, append them after the image name:

```sh
docker run --rm -p 8545:8545 anvil-polkadot \
  --block-time 1 \
  --accounts 5
```

### Using a specific release

If you want the container to reflect a specific released version instead of your local checkout,
use the `RELEASE_REF` build argument (tag, branch, or commit):

```sh
docker build \
  -f crates/anvil-polkadot/docker/Dockerfile \
  --build-arg RELEASE_REF=stable \
  -t anvil-polkadot .
```

