"""Open Ontologies Lite: a lightweight, pip-installable Python bridge to the
Oxigraph RDF/OWL engine. No Rust toolchain, no compilation, prebuilt wheels only.
"""

from .engine import OntologyEngine, ValidationResult, resolve_format
from .kgcl import ChangeSet, kgcl_diff

__version__ = "0.3.0"
__all__ = [
    "OntologyEngine",
    "ValidationResult",
    "resolve_format",
    "ChangeSet",
    "kgcl_diff",
    "__version__",
]

# AlignmentIndex needs the optional [align] extra (hnswlib); export it only when
# importable so the base package stays dependency-light.
try:  # pragma: no cover
    from .align import AlignmentIndex, Candidate  # noqa: F401
    __all__ += ["AlignmentIndex", "Candidate"]
except ImportError:  # pragma: no cover
    pass
