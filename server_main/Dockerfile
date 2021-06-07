FROM golang:1.16-alpine as build

WORKDIR /mk48

COPY go.mod go.sum ./
RUN go mod download

COPY ../server/server server/

WORKDIR /mk48/server
RUN CGO_ENABLED=0 go install -ldflags "-s -w"

FROM alpine:latest as run

WORKDIR /mk48
COPY --from=build /go/bin/server /mk48

ENTRYPOINT ["./server"]
