package main

import (
	"log"
	"net"

	pb "github.com/masallsome/masSentinel/mas-sentinel-controller/api/v1"
	"github.com/masallsome/masSentinel/mas-sentinel-controller/pkg/server"
	"google.golang.org/grpc"
)

func main() {
	lis, err := net.Listen("tcp", ":50051")
	if err != nil {
		log.Fatalf("failed to listen: %v", err)
	}
	s := grpc.NewServer()
	pb.RegisterConfigServiceServer(s, &server.ConfigServer{})
	log.Printf("server listening at %v", lis.Addr())
	if err := s.Serve(lis); err != nil {
		log.Fatalf("failed to serve: %v", err)
	}
}
