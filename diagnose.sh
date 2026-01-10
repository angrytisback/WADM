#!/bin/bash
echo "=== WADM Diagnostic Tool ==="
echo "1. Checking if wadm process is running..."
if pgrep -x "wadm" > /dev/null
then
    echo "WADM is RUNNING (PID: $(pgrep -x wadm))"
else
    echo "WADM is NOT RUNNING."
    echo "   Please start it with: sudo -E cargo run --release"
    exit 1
fi

echo "--------------------------------"
echo "2. Checking ports..."
if sudo ss -tulpn | grep ':8168' > /dev/null
then
    echo "Port 8168 is OPEN and listening."
else
    echo "Port 8168 is NOT listening. Application might be starting up or failed to bind."
    exit 1
fi

echo "--------------------------------"
echo "3. Testing Local Connection..."
curl -v http://localhost:8168 > /dev/null 2>&1
RET=$?
if [ $RET -eq 0 ]; then
    echo "Local connection SUCCESSFUL."
else
    echo "Local connection FAILED (curl exit code: $RET)."
    echo "  If local works but remote fails, it is a FIREWALL issue."
fi

echo "--------------------------------"
echo "4. Checking Firewall (UFW)..."
if command -v ufw > /dev/null; then
    sudo ufw status | grep 8168
else
    echo "UFW not installed/found. Check protection manually (iptables/AWS Security Groups)."
fi
