#!/bin/bash

docker network create --driver bridge devnet

docker run --rm -it \
  --network devnet \
  -v "/tmp/tendermint:/tendermint" \
  --name tendermint \
  tendermint/tendermint init
  
docker run --rm -it \
  --network devnet \
  -v "/tmp/tendermint:/tendermint" \
  --name tendermint \
  tendermint/tendermint node --proxy_app=tcp://rust-dev:26658
  
