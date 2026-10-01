# Pin the multi-platform image index so the helper stays native on arm64 and amd64 hosts.
FROM docker.io/library/docker:29.8.2-cli@sha256:b1805116a6a86cc591b5d5f60a910a0715cdcc9d18d866ad68b1457ead25c35c

RUN apk add --no-cache bash git jq nodejs npm
RUN npm install --global @devcontainers/cli@0.89.0
