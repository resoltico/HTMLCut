# Pin the multi-platform image index so the helper stays native on arm64 and amd64 hosts.
FROM docker.io/library/docker:29.7.2-cli@sha256:000bb62ff495f986c9f5578eb67cc2cb98b91138eda81d7762d5371eb8a497fe

RUN apk add --no-cache bash git jq nodejs npm
RUN npm install --global @devcontainers/cli@0.88.0
