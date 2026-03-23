#!/bin/bash
# Setup virtual CAN for testing without hardware
sudo modprobe vcan
sudo ip link add dev vcan0 type vcan
sudo ip link set up vcan0
echo "✅ vcan0 is up. Use: candump vcan0 / cansend vcan0 123#DEADBEEF"
