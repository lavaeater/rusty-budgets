#!/bin/bash

# Script to launch both server and client components of the Dioxus application
# This script can be used as an alternative to the dx serve command

# Set the working directory to the project root
cd "$(dirname "$0")"

# Function to handle cleanup on exit
cleanup() {
    echo "Shutting down processes..."
    if [ ! -z "$SERVER_PID" ]; then
        kill $SERVER_PID 2>/dev/null
    fi
    if [ ! -z "$CLIENT_PID" ]; then
        kill $CLIENT_PID 2>/dev/null
    fi
    exit 0
}

# Set up trap to catch Ctrl+C and other termination signals
trap cleanup SIGINT SIGTERM

# Start the server component
echo "Starting server component..."
cargo run --package desktop --no-default-features --features server &
SERVER_PID=$!

# Wait for the server to start (adjust the sleep time as needed)
echo "Waiting for server to initialize..."
sleep 3

# Start the client component
echo "Starting client component..."
cargo run --package desktop --no-default-features --features desktop &
CLIENT_PID=$!

# Wait for both processes to complete
echo "Both components are running. Press Ctrl+C to stop."
wait $SERVER_PID $CLIENT_PID

# Clean up on normal exit
cleanup