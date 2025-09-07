"""
d-vecDB Server Python Wrapper

Provides a Python interface to manage the d-vecDB server process.
"""

import os
import sys
import platform
import subprocess
import signal
import time
import tempfile
import logging
from pathlib import Path
from typing import Optional, Dict, Any, List
import threading

logger = logging.getLogger(__name__)


class DVecDBServer:
    """Python wrapper for the d-vecDB server binary."""
    
    def __init__(self, 
                 host: str = "127.0.0.1",
                 port: int = 8080,
                 grpc_port: int = 9090,
                 data_dir: Optional[str] = None,
                 log_level: str = "info",
                 config_file: Optional[str] = None):
        """
        Initialize d-vecDB server wrapper.
        
        Args:
            host: Server host address
            port: REST API port
            grpc_port: gRPC port  
            data_dir: Directory for data storage
            log_level: Logging level (debug, info, warn, error)
            config_file: Path to configuration file
        """
        self.host = host
        self.port = port
        self.grpc_port = grpc_port
        self.data_dir = data_dir or tempfile.mkdtemp(prefix="d-vecdb-")
        self.log_level = log_level
        self.config_file = config_file
        
        self._process: Optional[subprocess.Popen] = None
        self._temp_config: Optional[str] = None
        
        # Find the binary
        self._binary_path = self._find_binary()
        if not self._binary_path:
            raise RuntimeError(
                "d-vecDB server binary not found. "
                "The binary needs to be downloaded or built separately. "
                "Check the GitHub releases at https://github.com/rdmurugan/d-vecDB/releases "
                "or build from source."
            )
    
    def _find_binary(self) -> Optional[Path]:
        """Find the d-vecDB server binary."""
        
        # First, try to find in the package installation
        package_dir = Path(__file__).parent
        binaries_dir = package_dir / "binaries"
        
        # Determine binary name based on platform
        system = platform.system().lower()
        machine = platform.machine().lower()
        
        # Map platform/architecture to binary names
        if system == "darwin":  # macOS
            if machine in ["arm64", "aarch64"]:
                binary_name = "vectordb-server-darwin-arm64"
            else:
                binary_name = "vectordb-server-darwin-x64"
        elif system == "linux":
            if machine in ["x86_64", "amd64"]:
                binary_name = "vectordb-server-linux-x64"
            else:
                binary_name = f"vectordb-server-linux-{machine}"
        elif system == "windows":
            binary_name = "vectordb-server-windows-x64.exe"
        else:
            # Fallback to generic binary name
            binary_name = "vectordb-server"
        
        # Check package binaries directory with platform-specific name
        binary_path = binaries_dir / binary_name
        if binary_path.exists() and binary_path.is_file():
            # Ensure binary has execute permissions
            self._ensure_executable(binary_path)
            return binary_path
        
        # Fallback to generic binary name
        fallback_path = binaries_dir / "vectordb-server"
        if fallback_path.exists() and fallback_path.is_file():
            # Ensure binary has execute permissions
            self._ensure_executable(fallback_path)
            return fallback_path
        
        # Check if binary is in PATH (but not the Python wrapper)
        import shutil
        system_binary = shutil.which("vectordb-server")
        if system_binary:
            # Make sure it's not the Python wrapper script
            try:
                with open(system_binary, 'r') as f:
                    first_line = f.readline()
                    if not first_line.startswith('#!/') or 'python' not in first_line:
                        return Path(system_binary)
            except (IOError, UnicodeDecodeError):
                # If it's not readable as text, it's likely a binary
                return Path(system_binary)
        
        # Check common installation locations
        common_paths = [
            Path.home() / ".local" / "bin" / binary_name,
            Path("/usr/local/bin") / binary_name,
            Path("/opt/homebrew/bin") / binary_name,
        ]
        
        for path in common_paths:
            if path.exists() and path.is_file():
                # Ensure binary has execute permissions
                self._ensure_executable(path)
                return path
        
        return None
    
    def _ensure_executable(self, binary_path: Path) -> None:
        """Ensure the binary file has execute permissions."""
        try:
            import stat
            current_mode = binary_path.stat().st_mode
            # Add execute permissions for owner, group, and others
            new_mode = current_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH
            if current_mode != new_mode:
                binary_path.chmod(new_mode)
                logger.info(f"Set execute permissions on binary: {binary_path}")
        except Exception as e:
            logger.warning(f"Failed to set execute permissions on {binary_path}: {e}")
    
    def _create_config(self) -> str:
        """Create a temporary configuration file."""
        
        config_content = f"""
[server]
host = "{self.host}"
rest_port = {self.port}
grpc_port = {self.grpc_port}
metrics_port = 9091
log_level = "{self.log_level}"

[storage]
data_dir = "{self.data_dir}"
"""
        
        # Create temporary config file
        fd, temp_path = tempfile.mkstemp(suffix='.toml', prefix='d-vecdb-')
        try:
            with os.fdopen(fd, 'w') as f:
                f.write(config_content.strip())
        except Exception:
            os.close(fd)
            raise
        
        return temp_path
    
    def start(self, background: bool = True, timeout: int = 30) -> bool:
        """
        Start the d-vecDB server.
        
        Args:
            background: Run in background (default: True)
            timeout: Timeout in seconds to wait for server to start
            
        Returns:
            True if server started successfully
        """
        if self.is_running():
            logger.warning("Server is already running")
            return True
        
        # Create data directory if it doesn't exist
        os.makedirs(self.data_dir, exist_ok=True)
        
        # Build command 
        if self.config_file:
            # Use config file if provided
            cmd = [
                str(self._binary_path),
                "--config", self.config_file,
            ]
        else:
            # Use direct arguments
            # Convert localhost to IP address for server compatibility
            host_ip = "127.0.0.1" if self.host == "localhost" else self.host
            cmd = [
                str(self._binary_path),
                "--host", host_ip,
                "--rest-port", str(self.port),
                "--grpc-port", str(self.grpc_port),
                "--metrics-port", str(self.grpc_port + 1),  # Use grpc_port + 1 for metrics
                "--data-dir", self.data_dir,
                "--log-level", self.log_level,
            ]
        
        logger.info(f"Starting d-vecDB server: {' '.join(cmd)}")
        
        try:
            # Start the process
            if background:
                self._process = subprocess.Popen(
                    cmd,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE,
                    env={**os.environ, "RUST_LOG": self.log_level}
                )
            else:
                self._process = subprocess.Popen(
                    cmd,
                    env={**os.environ, "RUST_LOG": self.log_level}
                )
            
            # Wait for server to start
            if background:
                return self._wait_for_startup(timeout)
            else:
                # If not background, wait for process to complete
                self._process.wait()
                return self._process.returncode == 0
                
        except Exception as e:
            logger.error(f"Failed to start server: {e}")
            return False
    
    def _wait_for_startup(self, timeout: int) -> bool:
        """Wait for the server to start up and be ready."""
        import time
        import socket
        
        start_time = time.time()
        
        while time.time() - start_time < timeout:
            # Check if process is still running
            if self._process and self._process.poll() is not None:
                logger.error("Server process terminated unexpectedly")
                return False
            
            # Try to connect to the server
            try:
                with socket.create_connection((self.host, self.port), timeout=1):
                    logger.info("Server started successfully")
                    return True
            except (socket.error, OSError):
                time.sleep(0.5)
        
        logger.error(f"Server failed to start within {timeout} seconds")
        return False
    
    def stop(self, timeout: int = 10) -> bool:
        """
        Stop the d-vecDB server.
        
        Args:
            timeout: Timeout in seconds to wait for graceful shutdown
            
        Returns:
            True if server stopped successfully
        """
        if not self.is_running():
            logger.warning("Server is not running")
            return True
        
        logger.info("Stopping d-vecDB server")
        
        try:
            # Send SIGTERM for graceful shutdown
            self._process.terminate()
            
            # Wait for graceful shutdown
            try:
                self._process.wait(timeout=timeout)
                logger.info("Server stopped gracefully")
            except subprocess.TimeoutExpired:
                logger.warning("Graceful shutdown timeout, forcing termination")
                self._process.kill()
                self._process.wait()
                logger.info("Server forcefully terminated")
            
            self._process = None
            
            # Clean up temporary config file
            if self._temp_config and os.path.exists(self._temp_config):
                os.unlink(self._temp_config)
                self._temp_config = None
            
            return True
            
        except Exception as e:
            logger.error(f"Failed to stop server: {e}")
            return False
    
    def restart(self, timeout: int = 30) -> bool:
        """Restart the server."""
        logger.info("Restarting d-vecDB server")
        
        if not self.stop():
            return False
        
        time.sleep(1)  # Brief pause
        
        return self.start(timeout=timeout)
    
    def is_running(self) -> bool:
        """Check if the server is running."""
        return self._process is not None and self._process.poll() is None
    
    def get_status(self) -> Dict[str, Any]:
        """Get server status information."""
        status = {
            "running": self.is_running(),
            "host": self.host,
            "port": self.port,
            "grpc_port": self.grpc_port,
            "data_dir": self.data_dir,
            "log_level": self.log_level,
            "binary_path": str(self._binary_path) if self._binary_path else None,
            "pid": self._process.pid if self._process else None,
        }
        
        if self.is_running():
            import socket
            try:
                # Test REST API connection
                with socket.create_connection((self.host, self.port), timeout=1):
                    status["rest_api_accessible"] = True
            except (socket.error, OSError):
                status["rest_api_accessible"] = False
            
            try:
                # Test gRPC connection
                with socket.create_connection((self.host, self.grpc_port), timeout=1):
                    status["grpc_api_accessible"] = True
            except (socket.error, OSError):
                status["grpc_api_accessible"] = False
        
        return status
    
    def get_logs(self, lines: int = 100) -> List[str]:
        """Get recent server logs."""
        if not self.is_running() or not self._process:
            return []
        
        try:
            # This is a simplified implementation
            # In a real scenario, you'd want to read from log files
            if self._process.stdout:
                return ["Logs would be available here"]
            else:
                return ["Server running in foreground mode, no captured logs"]
        except Exception as e:
            logger.error(f"Failed to get logs: {e}")
            return []
    
    def __enter__(self):
        """Context manager entry."""
        self.start()
        return self
    
    def __exit__(self, exc_type, exc_val, exc_tb):
        """Context manager exit."""
        self.stop()
    
    def __del__(self):
        """Destructor to ensure cleanup."""
        if self.is_running():
            self.stop()