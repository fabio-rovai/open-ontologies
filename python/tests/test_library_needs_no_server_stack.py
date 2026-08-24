"""The library API must import without the MCP server stack.

`mcp` is imported in exactly one module, server.py. Requiring it as a core
dependency made every consumer of the library install an MCP server they had
not asked for, and it is not merely weight: `mcp` pulls a starlette major that
current fastapi does not accept, so installing this package into a project
using fastapi removes fastapi. Measured against semantica 0.6.6, which declares
`fastapi>=0.109.2`: installing 0.4.0 uninstalled it and every import of
`semantica/explorer` failed afterwards.
"""

import subprocess
import sys


LIBRARY_IMPORTS = (
    "from open_ontologies_lite import OntologyEngine, vocab_check, resolve_format; "
    "OntologyEngine().load("
    "'<http://x/A> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x/B> .', "
    "'ntriples'); "
    "print('ok')"
)


def test_the_library_api_imports_with_mcp_unavailable():
    """Run in a fresh process with `mcp` forced to be missing."""
    # `sys.modules[name] = None` makes `import name` raise ImportError, which
    # is what an uninstalled package looks like from inside the interpreter.
    blocker = "import sys; sys.modules['mcp'] = None\n" 
    result = subprocess.run(
        [sys.executable, "-c", blocker + LIBRARY_IMPORTS],
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0, result.stderr
    assert "ok" in result.stdout


def test_the_server_module_says_which_extra_it_needs():
    import open_ontologies_lite

    assert "mcp" not in getattr(open_ontologies_lite, "__all__", [])
