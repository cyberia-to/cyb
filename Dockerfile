# Deno for deps + task runner, Node for rspack runtime
FROM denoland/deno:2 as build

# Install Node.js (rspack/eslint binaries are Node scripts)
RUN apt-get update && apt-get install -y curl && \
    curl -fsSL https://deb.nodesource.com/setup_22.x | bash - && \
    apt-get install -y nodejs && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/app
COPY package.json deno.json ./
RUN deno install

COPY . .
RUN deno task build
