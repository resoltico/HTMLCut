FROM docker:29-cli

RUN apk add --no-cache bash git jq nodejs npm
RUN npm install --global @devcontainers/cli@0.86.0
