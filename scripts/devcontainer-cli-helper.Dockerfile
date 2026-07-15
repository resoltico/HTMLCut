FROM docker.io/library/docker:29.6.1-cli@sha256:862099ada15c669000bef53aa4cb9d821262829f45b0dda2159ccb276443043b

RUN apk add --no-cache bash git jq nodejs npm
RUN npm install --global @devcontainers/cli@0.87.0
