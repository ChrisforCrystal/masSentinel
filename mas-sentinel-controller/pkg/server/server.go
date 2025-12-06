package server

import (
    "fmt"
    "time"
    pb "github.com/masallsome/masSentinel/mas-sentinel-controller/api/v1"
)

type ConfigServer struct {
    pb.UnimplementedConfigServiceServer
}

func (s *ConfigServer) WatchConfig(req *pb.WatchRequest, stream pb.ConfigService_WatchConfigServer) error {
    fmt.Printf("Client connected: %s\n", req.AppName)
    
    // Send initial config
    err := stream.Send(&pb.ConfigResponse{
        FlowRules: []*pb.FlowRule{
            {
                Resource: "google.com",
                Count: 1.0,
                Grade: 1,
                ControlBehavior: 0,
            },
        },
    })
    if err != nil {
        return err
    }

    // Keep stream open
    for {
        time.Sleep(10 * time.Second)
        // In real app, wait for config changes
    }
}
