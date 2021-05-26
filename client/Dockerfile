FROM node:14-alpine

RUN apk update && apk add bash

WORKDIR /mk48/client

COPY client/svelte.config.js ./
COPY client/package*.json ./

RUN npm install

COPY client ./

EXPOSE 3000
EXPOSE 24678

ENV HOST=0.0.0.0

CMD [ "npm", "run", "dev" ]
