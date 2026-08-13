FROM docker.io/library/node:20-slim
WORKDIR /app
# node_modules is copied from the host install rather than fetched here:
# this container's network cannot reach registry.npmjs.org reliably
# (observed empirically; unrelated to package size or --omit=dev).
COPY frontend/package.json ./
COPY frontend/node_modules ./node_modules
COPY frontend/server.mjs ./
ENV PORT=3000
EXPOSE 3000
CMD ["node", "server.mjs"]
