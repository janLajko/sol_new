#!/bin/bash

# Redis Management Script
# Function: Start, stop, restart Redis server

# Color output functions
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

# Redis related configuration
REDIS_SERVER_CMD="redis-server"
REDIS_CONFIG="/etc/redis/redis.conf" # If you have a specific config file, specify it here
REDIS_PORT=6379  # Default Redis port

# Logging function
log() {
    echo -e "${GREEN}[$(date +'%Y-%m-%d %H:%M:%S')]${NC} $1"
}

# Error logging function
error_log() {
    echo -e "${RED}[$(date +'%Y-%m-%d %H:%M:%S')] Error:${NC} $1" >&2
}

# Warning logging function
warning_log() {
    echo -e "${YELLOW}[$(date +'%Y-%m-%d %H:%M:%S')] Warning:${NC} $1"
}

# Check if Redis is running - improved to use more reliable detection
check_redis_running() {
    # Method 1: Check using netstat/ss for the Redis port
    if command -v netstat &> /dev/null; then
        netstat -tuln | grep -q ":${REDIS_PORT}"
        if [ $? -eq 0 ]; then
            return 0  # Redis is running
        fi
    elif command -v ss &> /dev/null; then
        ss -tuln | grep -q ":${REDIS_PORT}"
        if [ $? -eq 0 ]; then
            return 0  # Redis is running
        fi
    fi
    
    # Method 2: Use ps to find Redis
    ps -ef | grep "redis-server" | grep -v grep &> /dev/null
    return $?
}

# Get Redis process ID - improved to handle different process names
get_redis_pid() {
    pid=$(ps -ef | grep "redis-server" | grep -v grep | awk '{print $2}')
    if [ -z "$pid" ]; then
        error_log "Redis process not found"
        return 1
    fi
    echo "$pid"
    return 0
}

# Start Redis
start_redis() {
    if check_redis_running; then
        warning_log "Redis is already running"
        return 0
    fi
    
    log "Starting Redis server..."
    
    if [ -f "$REDIS_CONFIG" ]; then
        $REDIS_SERVER_CMD $REDIS_CONFIG &
    else
        $REDIS_SERVER_CMD &
    fi
    
    sleep 1
    
    if check_redis_running; then
        pid=$(get_redis_pid)
        log "Redis server started, process ID: $pid"
        return 0
    else
        error_log "Failed to start Redis server"
        return 1
    fi
}

# Stop Redis
stop_redis() {
    if ! check_redis_running; then
        warning_log "Redis server is not running"
        return 0
    fi
    
    pid=$(get_redis_pid)
    if [ $? -ne 0 ]; then
        return 1
    fi
    
    log "Stopping Redis server (process ID: $pid)..."
    kill $pid
    
    # Wait for process termination
    for i in {1..5}; do
        if ! check_redis_running; then
            log "Redis server stopped"
            return 0
        fi
        sleep 1
    done
    
    # If process is still running, use force termination
    warning_log "Redis server not responding in time, attempting forced termination..."
    kill -9 $pid
    
    if ! check_redis_running; then
        log "Redis server forcefully stopped"
        return 0
    else
        error_log "Unable to stop Redis server"
        return 1
    fi
}

# Restart Redis with port check
restart_redis() {
    log "Restarting Redis server..."
    
    # Check if port is in use by a different process
    if check_redis_running; then
        pid=$(get_redis_pid)
        if [ -z "$pid" ]; then
            # Port is in use but not by redis-server - could be another process
            error_log "Port ${REDIS_PORT} is in use by another process. Cannot restart Redis."
            return 1
        fi
    fi
    
    stop_redis
    if [ $? -ne 0 ]; then
        error_log "Unable to stop Redis server, restart failed"
        return 1
    fi
    
    start_redis
    if [ $? -ne 0 ]; then
        error_log "Unable to start Redis server, restart failed"
        return 1
    fi
    
    log "Redis server successfully restarted"
    return 0
}

# Display Redis status with improved detection
status_redis() {
    if check_redis_running; then
        pid=$(get_redis_pid)
        log "Redis server is running, process ID: $pid"
        
        # Display port information
        if command -v netstat &> /dev/null; then
            echo "---------- Redis Port Information ----------"
            netstat -tuln | grep ":${REDIS_PORT}"
        elif command -v ss &> /dev/null; then
            echo "---------- Redis Port Information ----------"
            ss -tuln | grep ":${REDIS_PORT}"
        fi
        
        # If redis-cli is installed, get more Redis information
        if command -v redis-cli &> /dev/null; then
            echo "---------- Redis Information ----------"
            redis-cli info | grep -E 'redis_version|uptime_in_days|connected_clients|used_memory_human|total_connections_received'
        fi
    else
        warning_log "Redis server is not running"
    fi
}

# Find Redis process with more detailed information
find_redis_process() {
    log "Finding Redis process..."
    ps -ef | grep "redis-server" | grep -v grep
    
    echo "---------- Port ${REDIS_PORT} Usage ----------"
    if command -v netstat &> /dev/null; then
        netstat -tuln | grep ":${REDIS_PORT}"
    elif command -v ss &> /dev/null; then
        ss -tuln | grep ":${REDIS_PORT}"
    fi
    
    if ! check_redis_running; then
        warning_log "No running Redis process found"
    fi
}

# Show usage help
show_help() {
    echo "Redis Management Script"
    echo "Usage: $0 {start|stop|restart|status|find|help}"
    echo ""
    echo "Commands:"
    echo "  start    Start Redis server"
    echo "  stop     Stop Redis server"
    echo "  restart  Restart Redis server"
    echo "  status   Display Redis server status"
    echo "  find     Find Redis process"
    echo "  help     Display this help information"
}

# Main function
main() {
    case "$1" in
        start)
            start_redis
            ;;
        stop)
            stop_redis
            ;;
        restart)
            restart_redis
            ;;
        status)
            status_redis
            ;;
        find)
            find_redis_process
            ;;
        help|--help|-h)
            show_help
            ;;
        *)
            show_help
            exit 1
            ;;
    esac
}

# Execute main function
main "$@"