"""
Command Line Interface for d-vecDB Server Python Package
"""

import sys
import argparse
import logging
import signal
import time
from pathlib import Path
from typing import Optional

from .server import DVecDBServer

# Setup logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)


def handle_signal(signum, frame):
    """Handle shutdown signals gracefully."""
    logger.info("Received shutdown signal, stopping server...")
    sys.exit(0)


def cmd_start(args):
    """Start the d-vecDB server."""
    
    # Create server instance
    server = DVecDBServer(
        host=args.host,
        port=args.port,
        grpc_port=args.grpc_port,
        data_dir=args.data_dir,
        log_level=args.log_level,
        config_file=args.config
    )
    
    # Setup signal handlers
    signal.signal(signal.SIGINT, handle_signal)
    signal.signal(signal.SIGTERM, handle_signal)
    
    print(f"🚀 Starting d-vecDB server...")
    print(f"   Host: {args.host}")
    print(f"   REST API: http://{args.host}:{args.port}")
    print(f"   gRPC API: {args.host}:{args.grpc_port}")
    print(f"   Data directory: {server.data_dir}")
    print(f"   Log level: {args.log_level}")
    print()
    
    try:
        if hasattr(args, 'daemon') and args.daemon:
            # Start in background
            if server.start(background=True):
                print("✅ Server started successfully in background")
                print(f"   PID: {server._process.pid}")
                print()
                print("Use the following commands to manage the server:")
                print("  d-vecdb-server stop    - Stop the server")
                print("  d-vecdb-server status  - Check server status")
                print("  d-vecdb-server logs    - View server logs")
            else:
                print("❌ Failed to start server")
                return 1
        else:
            # Start in foreground
            print("Press Ctrl+C to stop the server")
            print("-" * 40)
            
            try:
                if server.start(background=False):
                    return 0
                else:
                    print("❌ Server exited with error")
                    return 1
            except KeyboardInterrupt:
                print("\n🛑 Stopping server...")
                server.stop()
                print("✅ Server stopped")
                return 0
                
    except Exception as e:
        logger.error(f"Error starting server: {e}")
        return 1


def cmd_stop(args):
    """Stop the d-vecDB server."""
    
    # This is a simplified implementation
    # In a real scenario, you'd want to track PIDs in a file
    print("🛑 Stopping d-vecDB server...")
    
    import psutil
    
    # Find d-vecDB server processes
    stopped = False
    for proc in psutil.process_iter(['pid', 'name', 'cmdline']):
        try:
            if 'vectordb-server' in proc.info['name'] or \
               any('vectordb-server' in arg for arg in proc.info['cmdline'] or []):
                proc.terminate()
                proc.wait(timeout=10)
                print(f"✅ Stopped process {proc.info['pid']}")
                stopped = True
        except (psutil.NoSuchProcess, psutil.AccessDenied):
            continue
        except psutil.TimeoutExpired:
            proc.kill()
            print(f"✅ Forcefully stopped process {proc.info['pid']}")
            stopped = True
    
    if not stopped:
        print("⚠️  No running d-vecDB server processes found")
    
    return 0


def cmd_status(args):
    """Check server status."""
    
    print("📊 d-vecDB Server Status")
    print("=" * 30)
    
    import socket
    import psutil
    
    # Check for running processes
    running_procs = []
    for proc in psutil.process_iter(['pid', 'name', 'cmdline', 'status']):
        try:
            if 'vectordb-server' in proc.info['name'] or \
               any('vectordb-server' in arg for arg in proc.info['cmdline'] or []):
                running_procs.append(proc.info)
        except (psutil.NoSuchProcess, psutil.AccessDenied):
            continue
    
    if running_procs:
        print("✅ Server Status: RUNNING")
        for proc in running_procs:
            print(f"   PID: {proc['pid']}")
            print(f"   Status: {proc['status']}")
    else:
        print("❌ Server Status: STOPPED")
        return 0
    
    # Check port accessibility
    ports_to_check = [
        (args.host, args.port, "REST API"),
        (args.host, args.grpc_port, "gRPC API"),
        (args.host, 9091, "Metrics"),
    ]
    
    print("\n🌐 Port Status:")
    for host, port, service in ports_to_check:
        try:
            with socket.create_connection((host, port), timeout=2):
                print(f"   ✅ {service}: {host}:{port}")
        except (socket.error, OSError):
            print(f"   ❌ {service}: {host}:{port} (not accessible)")
    
    return 0


def cmd_logs(args):
    """Show server logs."""
    
    print("📋 d-vecDB Server Logs")
    print("=" * 30)
    
    # This is a placeholder implementation
    # In a real scenario, you'd read from actual log files
    print("Log viewing not implemented in this version.")
    print("Check system logs or run the server in foreground mode.")
    
    return 0


def cmd_version(args):
    """Show version information."""
    from . import __version__
    
    print(f"d-vecDB Server Python Package v{__version__}")
    
    # Try to get binary version without failing
    try:
        server = DVecDBServer()
        if server._binary_path:
            print(f"Binary location: {server._binary_path}")
            
            # Check if binary exists and has execute permissions
            import stat
            try:
                binary_stat = server._binary_path.stat()
                has_exec = bool(binary_stat.st_mode & stat.S_IXUSR)
                print(f"Binary permissions: {'executable' if has_exec else 'not executable'}")
                print(f"Binary size: {binary_stat.st_size / 1024 / 1024:.1f} MB")
            except Exception as e:
                print(f"Binary stat error: {e}")
            
            # Try version check
            try:
                import subprocess
                result = subprocess.run([str(server._binary_path), "--version"], 
                                      capture_output=True, text=True, timeout=5)
                if result.returncode == 0:
                    print(f"Binary version: {result.stdout.strip()}")
                else:
                    print(f"Binary: Found but version check failed (exit code: {result.returncode})")
                    if result.stderr:
                        print(f"Error output: {result.stderr.strip()}")
            except subprocess.TimeoutExpired:
                print("Binary: Found but version check timed out")
            except PermissionError:
                print("Binary: Found but permission denied - trying to fix permissions...")
                server._ensure_executable(server._binary_path)
                print("Execute permissions set - please try again")
            except Exception as e:
                print(f"Binary: Found but not accessible - {type(e).__name__}: {e}")
        else:
            print("Binary: Not found")
    except RuntimeError as e:
        print(f"Binary: Not available - {str(e)}")
    
    return 0


def main():
    """Main CLI entry point."""
    
    parser = argparse.ArgumentParser(
        description="d-vecDB Server Python Package",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  d-vecdb-server start                 # Start server in foreground
  d-vecdb-server start --daemon        # Start server in background  
  d-vecdb-server start --port 8081     # Start on custom port
  d-vecdb-server stop                  # Stop running server
  d-vecdb-server status                # Check server status
        """
    )
    
    # Global options
    parser.add_argument("--host", default="127.0.0.1", 
                       help="Server host (default: 127.0.0.1)")
    parser.add_argument("--port", type=int, default=8080, 
                       help="REST API port (default: 8080)")
    parser.add_argument("--grpc-port", type=int, default=9090, 
                       help="gRPC port (default: 9090)")
    parser.add_argument("--data-dir", 
                       help="Data directory (default: temporary directory)")
    parser.add_argument("--config", 
                       help="Configuration file path")
    parser.add_argument("--log-level", default="info", 
                       choices=["debug", "info", "warn", "error"],
                       help="Log level (default: info)")
    
    # Subcommands
    subparsers = parser.add_subparsers(dest="command", help="Available commands")
    
    # Start command
    start_parser = subparsers.add_parser("start", help="Start the server")
    start_parser.add_argument("--daemon", action="store_true", 
                             help="Run in background")
    
    # Stop command
    subparsers.add_parser("stop", help="Stop the server")
    
    # Status command  
    subparsers.add_parser("status", help="Check server status")
    
    # Logs command
    logs_parser = subparsers.add_parser("logs", help="Show server logs")
    logs_parser.add_argument("--lines", type=int, default=100,
                            help="Number of log lines to show")
    
    # Version command
    subparsers.add_parser("version", help="Show version information")
    
    # Parse arguments
    args = parser.parse_args()
    
    # Handle commands
    if args.command == "start":
        return cmd_start(args)
    elif args.command is None:
        # No subcommand provided, show help
        parser.print_help()
        return 1
    elif args.command == "stop":
        return cmd_stop(args)
    elif args.command == "status":
        return cmd_status(args)
    elif args.command == "logs":
        return cmd_logs(args)
    elif args.command == "version":
        return cmd_version(args)
    else:
        parser.print_help()
        return 1


if __name__ == "__main__":
    sys.exit(main())