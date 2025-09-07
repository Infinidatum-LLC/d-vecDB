"""
d-vecDB Server Python Package

This package provides a high-performance vector database server with embedded binaries
for multiple platforms.
"""

__version__ = "0.1.5"
__author__ = "Durai"
__email__ = "durai@infinidatum.com"

from .server import DVecDBServer
from .cli import main

__all__ = ["DVecDBServer", "main"]