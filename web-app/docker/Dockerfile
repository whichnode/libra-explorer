#########################################
#              FIRST STAGE              #
#########################################

FROM node:20.12.2 AS builder
WORKDIR /app

ARG CI_COMMIT_SHA=""
ENV VITE_CI_COMMIT_SHA $CI_COMMIT_SHA

COPY ./ ./
# RUN npm ci
RUN npm install

RUN npm run build

#########################################
#             SECOND STAGE              #
#########################################

FROM nginx:1.25

COPY --from=builder /app/dist /app
COPY docker/nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 80
