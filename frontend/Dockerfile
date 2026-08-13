FROM docker.io/library/node:20-slim
WORKDIR /app
# node_modules is copied from the host install rather than fetched here:
# this container's network cannot reach registry.npmjs.org reliably
# (observed empirically; unrelated to package size or --omit=dev).
COPY frontend/package.json ./
COPY frontend/node_modules ./node_modules
COPY frontend/index.html frontend/vite.config.ts frontend/tsconfig.json ./
COPY frontend/src ./src
COPY frontend/public ./public
EXPOSE 3000
CMD ["npx", "vite"]
