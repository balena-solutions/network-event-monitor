#!/bin/bash

INTERFACE=$1
ACTION=$2

# Set the dbus system bus address
# Security Warning: This script uses the system bus, which can have security implications. 
# Ensure that the system bus is properly secured and that only trusted applications can send signals to it.
export DBUS_SYSTEM_BUS_ADDRESS=unix:path=/run/dbus/system_bus_socket

# Publish the interface and action to dbus
dbus-send \
  --system \
  --type=signal \
  /com/example/NetworkEvent \
  com.example.NetworkEvent.InterfaceAction \
  string:"$INTERFACE" \
  string:"$ACTION"