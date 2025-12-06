.PHONY: proto build

proto:
	protoc --go_out=. --go_opt=paths=source_relative \
	--go-grpc_out=. --go-grpc_opt=paths=source_relative \
	api/v1/config.proto

build:
	go build -o bin/controller main.go
：