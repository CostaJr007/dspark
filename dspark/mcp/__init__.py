"""
MCP package providing FastMCP server and tool registration for DSpark Dual-Engine.
"""

from .server import create_mcp_server, run_mcp_server

__all__ = ["create_mcp_server", "run_mcp_server"]
