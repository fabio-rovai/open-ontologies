"""HNSW alignment index: an approximate-nearest-neighbour candidate generator.

This is a *primitive*, deliberately MCP-native: the package owns the index, the
caller owns the vectors. Open Ontologies Lite never generates embeddings and never
calls a model. You bring vectors (from whatever embedding model your orchestrator
uses); this builds an HNSW index over them and returns nearest-neighbour candidates
for ontology alignment. Adjudicating those candidates into actual mappings is the
orchestrator's job, not the server's.

Requires the optional extra:  pip install "open-ontologies-lite[align]"

Reference: hnswlib, https://github.com/nmslib/hnswlib
"""
from __future__ import annotations

from dataclasses import dataclass


def _require_hnswlib():
    try:
        import hnswlib  # noqa: F401
        return hnswlib
    except ImportError as exc:  # pragma: no cover - exercised only without the extra
        raise ImportError(
            "AlignmentIndex needs the optional 'align' extra. "
            'Install with: pip install "open-ontologies-lite[align]"'
        ) from exc


@dataclass
class Candidate:
    id: str
    score: float  # similarity in [0, 1] for cosine space (1.0 = identical direction)


class AlignmentIndex:
    """An HNSW index over caller-supplied vectors, keyed by string ids.

    Typical use::

        idx = AlignmentIndex(dim=384)
        idx.add("flw:PC-BAK", vec_bakery)        # vectors come from YOUR embedder
        idx.add("FOODON:00001626", vec_foodon)
        idx.build()
        idx.query(vec_query, k=5)                # -> [Candidate(id, score), ...]
    """

    def __init__(self, dim: int, space: str = "cosine", max_elements: int = 100_000,
                 ef_construction: int = 200, M: int = 16) -> None:
        self._hnswlib = _require_hnswlib()
        self.dim = dim
        self.space = space
        self._index = self._hnswlib.Index(space=space, dim=dim)
        self._index.init_index(max_elements=max_elements, ef_construction=ef_construction, M=M)
        self._ids: list[str] = []
        self._built = False

    def add(self, id: str, vector) -> int:
        """Add one (id, vector). Returns the internal integer label."""
        if len(vector) != self.dim:
            raise ValueError(f"vector length {len(vector)} != index dim {self.dim}")
        label = len(self._ids)
        self._index.add_items([list(vector)], [label])
        self._ids.append(id)
        self._built = True
        return label

    def add_many(self, items) -> int:
        """Add an iterable of (id, vector). Returns the count added."""
        n = 0
        for id, vec in items:
            self.add(id, vec)
            n += 1
        return n

    def build(self, ef: int = 50) -> None:
        """Set the query-time ef parameter (higher = more accurate, slower)."""
        self._index.set_ef(ef)
        self._built = True

    def query(self, vector, k: int = 5) -> list[Candidate]:
        """Return up to k nearest candidates for a query vector, best first."""
        if not self._ids:
            return []
        self._index.set_ef(max(k * 4, 50))
        labels, distances = self._index.knn_query([list(vector)], k=min(k, len(self._ids)))
        out = []
        for lbl, dist in zip(labels[0], distances[0]):
            # cosine space: hnswlib distance = 1 - cosine_similarity
            score = 1.0 - float(dist) if self.space == "cosine" else -float(dist)
            out.append(Candidate(id=self._ids[int(lbl)], score=round(score, 4)))
        return out

    def __len__(self) -> int:
        return len(self._ids)
