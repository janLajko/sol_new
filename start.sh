#!/bin/bash

# Script name: restart_sol_new.sh
# Description: Start cargo run and automatically restart when the process exits

# Set up log file
LOG_FILE="sol_new_restart.log"
MAX_RESTARTS=100  # Maximum number of restarts to prevent infinite restart loops
RESTART_DELAY=2   # Seconds to wait before restarting

# Initialize counter
restart_count=0

# Use English date format (Month Day HH:MM:SS Year)
echo "$(date "+%b %d %H:%M:%S %Y"): Started monitoring sol_new process" | tee -a "$LOG_FILE"

while [ $restart_count -lt $MAX_RESTARTS ]; do
    # Increment counter
    restart_count=$((restart_count + 1))
    
    echo "$(date "+%b %d %H:%M:%S %Y"): Starting sol_new process (attempt #$restart_count)" | tee -a "$LOG_FILE"
    
    # Start the cargo run process
    cargo run
    
    # Get exit status
    exit_status=$?
    
    echo "$(date "+%b %d %H:%M:%S %Y"): sol_new process exited with status code: $exit_status" | tee -a "$LOG_FILE"
    
    # Wait a few seconds before restarting
    echo "$(date "+%b %d %H:%M:%S %Y"): Waiting $RESTART_DELAY seconds before restarting..." | tee -a "$LOG_FILE"
    sleep $RESTART_DELAY
done

echo "$(date "+%b %d %H:%M:%S %Y"): Maximum restart count ($MAX_RESTARTS) reached, no further restarts." | tee -a "$LOG_FILE"