#!/usr/bin/env python3
"""
d-vecDB Server Python Package - Custom installation hooks
Includes pre-built binaries for multiple platforms
"""

import os
import platform
from pathlib import Path
from setuptools import setup
from setuptools.command.install import install
from setuptools.command.develop import develop
import requests

# Package metadata
VERSION = "0.1.7"
REPO_URL = "https://github.com/rdmurugan/d-vecDB"
BINARY_NAME = "vectordb-server"

# Platform detection
def get_platform():
    """Detect the current platform and return the appropriate binary suffix."""
    system = platform.system().lower()
    machine = platform.machine().lower()
    
    if system == "darwin":  # macOS
        if machine in ["arm64", "aarch64"]:
            return "macos-arm64"
        else:
            return "macos-x64"
    elif system == "linux":
        if machine in ["x86_64", "amd64"]:
            return "linux-musl-x64"
        else:
            raise RuntimeError(f"Unsupported Linux architecture: {machine}")
    elif system == "windows":
        if machine in ["x86_64", "amd64"]:
            return "windows-x64.exe"
        else:
            raise RuntimeError(f"Unsupported Windows architecture: {machine}")
    else:
        raise RuntimeError(f"Unsupported operating system: {system}")

def download_binary(version, platform_suffix):
    """Download the appropriate binary for the current platform."""
    binary_filename = f"{BINARY_NAME}-{platform_suffix}"
    download_url = f"{REPO_URL}/releases/download/v{version}/{binary_filename}"
    
    print(f"Downloading {binary_filename} from {download_url}")
    
    # Create binaries directory
    binaries_dir = Path(__file__).parent / "d_vecdb_server" / "binaries"
    binaries_dir.mkdir(parents=True, exist_ok=True)
    
    # Determine output filename
    if platform_suffix.endswith(".exe"):
        output_filename = f"{BINARY_NAME}.exe"
    else:
        output_filename = BINARY_NAME
    
    output_path = binaries_dir / output_filename
    
    # Download binary
    try:
        response = requests.get(download_url, stream=True)
        response.raise_for_status()
        
        with open(output_path, 'wb') as f:
            for chunk in response.iter_content(chunk_size=8192):
                f.write(chunk)
        
        # Make executable on Unix systems
        if not platform_suffix.endswith(".exe"):
            os.chmod(output_path, 0o755)
        
        print(f"✅ Binary downloaded to {output_path}")
        return output_path
        
    except requests.exceptions.RequestException as e:
        print(f"❌ Failed to download binary: {e}")
        return None

class CustomInstallCommand(install):
    """Custom install command to download binary after installation."""
    
    def run(self):
        # Run the standard installation
        install.run(self)
        
        # Download the appropriate binary
        try:
            platform_suffix = get_platform()
            binary_path = download_binary(VERSION, platform_suffix)
            
            if binary_path and binary_path.exists():
                print(f"✅ d-vecDB server binary installed successfully")
                print(f"   Location: {binary_path}")
                print(f"   Run with: python -m d_vecdb_server")
            else:
                print("⚠️  Binary download failed, but package installed")
                print("   You can download manually from:")
                print(f"   {REPO_URL}/releases")
                
        except Exception as e:
            print(f"⚠️  Warning: {e}")
            print("   Package installed but binary unavailable for this platform")

class CustomDevelopCommand(develop):
    """Custom develop command."""
    
    def run(self):
        develop.run(self)
        
        # Also download binary for development
        try:
            platform_suffix = get_platform()
            download_binary(VERSION, platform_suffix)
        except Exception as e:
            print(f"⚠️  Warning during development setup: {e}")

# Use pyproject.toml for all package metadata, only custom commands here
if __name__ == "__main__":
    setup(
        cmdclass={
            "install": CustomInstallCommand,
            "develop": CustomDevelopCommand,
        },
    )