#!/usr/bin/env python3
import sys
import os
import signal
import socket
import json
import time

running = True

def signal_handler(signum, frame):
    global running
    running = False

signal.signal(signal.SIGTERM, signal_handler)
signal.signal(signal.SIGINT, signal_handler)

def parse_args():
    socket_path = None
    args = sys.argv[1:]
    for i in range(len(args)):
        if args[i] == "--socket" and i + 1 < len(args):
            socket_path = args[i + 1]
    return socket_path

def main():
    global running
    socket_path = parse_args()
    if not socket_path:
        print("Error: --socket <path> argument is required", file=sys.stderr)
        sys.exit(1)

    if os.path.exists(socket_path):
        try:
            os.remove(socket_path)
        except OSError:
            pass

    server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    server.bind(socket_path)
    server.listen(10)
    server.settimeout(1.0)

    start_time = time.time()
    print(f"Mock plugin started. Listening on UDS: {socket_path}", file=sys.stderr)

    while running:
        try:
            conn, _ = server.accept()
        except socket.timeout:
            continue
        except OSError:
            break

        try:
            conn.settimeout(5.0)
            file_obj = conn.makefile("rwb", buffering=0)
            line = file_obj.readline()
            if not line:
                conn.close()
                continue

            req = json.loads(line.decode("utf-8"))
            req_id = req.get("id")
            method = req.get("method")
            params = req.get("params") or {}

            response = None
            if method == "plugin.init":
                response = {
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": {
                        "status": "initialized",
                        "plugin": "wadm-mock-service",
                        "version": "1.0.0",
                        "uptime": round(time.time() - start_time, 2)
                    }
                }
            elif method == "ping":
                response = {
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": "pong"
                }
            elif method == "service.status":
                response = {
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": {
                        "service_name": "Mock Worker",
                        "status": "healthy",
                        "uptime_sec": round(time.time() - start_time, 1),
                        "items_processed": int(time.time() - start_time) * 12,
                        "cpu_usage": 0.42,
                        "memory_usage_mb": 14.8
                    }
                }
            elif method == "service.action":
                action = params.get("action", "unknown")
                response = {
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": {
                        "action_executed": action,
                        "success": True,
                        "timestamp": time.time()
                    }
                }
            elif method == "plugin.shutdown":
                response = {
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": "shutdown_ack"
                }
                running = False
            else:
                response = {
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "error": {
                        "code": -32601,
                        "message": f"Method '{method}' not found"
                    }
                }

            out_data = (json.dumps(response) + "\n").encode("utf-8")
            file_obj.write(out_data)
            conn.close()
        except Exception as e:
            print(f"Error handling request: {e}", file=sys.stderr)
            try:
                conn.close()
            except Exception:
                pass

    try:
        server.close()
    except Exception:
        pass

    if os.path.exists(socket_path):
        try:
            os.remove(socket_path)
        except OSError:
            pass

    print("Mock plugin terminated cleanly.", file=sys.stderr)

if __name__ == "__main__":
    main()
