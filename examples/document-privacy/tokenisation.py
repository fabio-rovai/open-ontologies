#!/usr/bin/env python3
"""
Swappable tokenisation: detection and custody are separate concerns.

The market splits along exactly this seam, so the code should too:

  DETECTION   finding the sensitive spans in free text.
              Regex (demo) | Microsoft Presidio (MIT, local) | a vendor's own.

  CUSTODY     holding the mapping and controlling reversal.
              Local JSON (demo only) | Skyflow (managed) | Databunker (self-hosted).

Skyflow sells both halves in one product. Open source makes you assemble them,
which is fine as long as the assembly is a config choice rather than a rewrite.

    ONTO_DETECTOR = regex | presidio | both
    ONTO_VAULT    = local | skyflow | databunker

Everything downstream depends only on `Tokeniser`, so swapping either half
changes nothing in the pipeline. That is the point: the tokeniser is an
interchangeable component, not an architectural commitment.

The one invariant every implementation must hold: **tokens are deterministic**.
The same value must produce the same token in every document, because that is
what lets a token double as a join key for exact-match entity resolution
without any component handling the raw value.
"""

from __future__ import annotations

import hashlib
import hmac
import json
import os
import pathlib
import re
from dataclasses import dataclass
from typing import Protocol


@dataclass(frozen=True)
class Span:
    """A detected sensitive span: character offsets plus what it looks like."""
    start: int
    end: int
    kind: str
    text: str


# --------------------------------------------------------------------------
# Detection
# --------------------------------------------------------------------------

class Detector(Protocol):
    name: str

    def detect(self, text: str) -> list[Span]:
        ...


class RegexDetector:
    """Pattern matching. Demonstration only.

    Honest about its limits: this finds email addresses, phone-shaped strings,
    titled names and identifier-shaped tokens. It will miss bare personal names,
    addresses, dates of birth, national identifiers, and anything not in
    English. Do not present this as PII detection.
    """

    name = "regex"

    PATTERNS = [
        ("EMAIL", re.compile(r"\b[\w.+-]+@[\w-]+\.[\w.]{2,}\b")),
        ("PHONE", re.compile(r"\b(?:\+\d{1,3}[ -]?)?(?:\(?\d{3,5}\)?[ -]?){2,3}\d{3,4}\b")),
        ("PERSON", re.compile(r"\b(?:Dr|Prof|Mr|Mrs|Ms)\.? [A-Z][a-z]+ [A-Z][a-z]+\b")),
        ("ID", re.compile(r"\b[A-Z]{2,4}-\d{4,}\b")),
    ]

    def detect(self, text: str) -> list[Span]:
        spans: list[Span] = []
        for kind, pattern in self.PATTERNS:
            for m in pattern.finditer(text):
                spans.append(Span(m.start(), m.end(), kind, m.group(0)))
        return _dedupe(spans)


class PresidioDetector:
    """Microsoft Presidio. MIT licensed, runs locally, no egress.

    This is the component that makes tokenisation real rather than illustrative:
    it recognises a broad set of entity types across locales, which is precisely
    where pattern matching fails.

    Install:  pip install presidio-analyzer && python -m spacy download en_core_web_lg
    """

    name = "presidio"

    DEFAULT_ENTITIES = [
        "PERSON", "EMAIL_ADDRESS", "PHONE_NUMBER", "LOCATION",
        "DATE_TIME", "NRP", "CREDIT_CARD", "IBAN_CODE", "IP_ADDRESS",
        "MEDICAL_LICENSE", "UK_NHS", "US_SSN",
    ]

    def __init__(self, entities: list[str] | None = None, language: str = "en"):
        try:
            from presidio_analyzer import AnalyzerEngine  # type: ignore
        except ImportError as e:
            raise RuntimeError(
                "presidio-analyzer is not installed. "
                "pip install presidio-analyzer && python -m spacy download en_core_web_lg"
            ) from e
        self._engine = AnalyzerEngine()
        self.entities = entities or self.DEFAULT_ENTITIES
        self.language = language

    def detect(self, text: str) -> list[Span]:
        results = self._engine.analyze(text=text, entities=self.entities, language=self.language)
        return _dedupe(
            [Span(r.start, r.end, r.entity_type, text[r.start:r.end]) for r in results]
        )


class CompositeDetector:
    """Run several detectors and merge their spans.

    Neither half is sufficient alone, which the test output makes obvious:
    Presidio finds names, dates and locations that no pattern will catch, and
    misses domain identifiers like DOC-20481 because it has no reason to know
    they are sensitive. Regex catches those and nothing else.

    Overlaps resolve to the longer span, so a Presidio PERSON covering a name
    wins over a narrower pattern hit on part of it.
    """

    def __init__(self, *detectors: Detector):
        self._detectors = [d for d in detectors if d is not None]
        self.name = "+".join(d.name for d in self._detectors)

    def detect(self, text: str) -> list[Span]:
        spans: list[Span] = []
        for d in self._detectors:
            spans.extend(d.detect(text))
        return _dedupe(spans)


def _dedupe(spans: list[Span]) -> list[Span]:
    """Drop overlaps, preferring the longer match, and return in reverse order.

    Reverse order matters: replacements are applied back-to-front so earlier
    offsets stay valid as the string is rewritten.
    """
    ordered = sorted(spans, key=lambda s: (s.start, -(s.end - s.start)))
    kept: list[Span] = []
    last_end = -1
    for s in ordered:
        if s.start >= last_end:
            kept.append(s)
            last_end = s.end
    return sorted(kept, key=lambda s: s.start, reverse=True)


# --------------------------------------------------------------------------
# Custody
# --------------------------------------------------------------------------

class VaultBackend(Protocol):
    name: str

    def tokenise(self, kind: str, value: str) -> str:
        ...

    def detokenise(self, token: str) -> str | None:
        ...

    def save(self) -> None:
        ...


class LocalVault:
    """Deterministic HMAC tokens with a JSON store. DEMONSTRATION ONLY.

    Two properties are real: tokens are deterministic, and reversal needs the
    store. Everything else that makes a vault a security control is absent.

    Be explicit about the risk, because it is the opposite of obvious: the
    store is a plaintext file mapping every token back to its original value.
    If it leaks it is a single artefact containing all the sensitive data in
    the corpus. That is worse than not tokenising, because it looks like a
    control while concentrating the exposure. Never ship this.
    """

    name = "local"

    def __init__(self, key: bytes | None = None, path: pathlib.Path | None = None):
        self.key = key or os.environ.get("ONTO_VAULT_KEY", "demo-key-not-for-production").encode()
        self.path = path or pathlib.Path(os.environ.get("ONTO_VAULT_PATH", "demo/derived/_vault.json"))
        self.map: dict[str, str] = {}
        if self.path.exists():
            try:
                self.map = json.loads(self.path.read_text())
            except Exception:
                self.map = {}

    def tokenise(self, kind: str, value: str) -> str:
        digest = hmac.new(self.key, value.encode(), hashlib.sha256).hexdigest()[:12]
        token = f"TOK_{kind}_{digest}"
        self.map[token] = value
        return token

    def detokenise(self, token: str) -> str | None:
        return self.map.get(token)

    def save(self) -> None:
        self.path.parent.mkdir(parents=True, exist_ok=True)
        self.path.write_text(json.dumps(self.map, indent=1))


class SkyflowVault:
    """Skyflow managed vault.

    Skyflow has no open-source edition; this speaks to the hosted API. Custody,
    rotation, access policy and audit are the vendor's, which is the reason to
    use it over anything here.

    Requires SKYFLOW_VAULT_URL, SKYFLOW_VAULT_ID and SKYFLOW_BEARER_TOKEN.
    Determinism must be configured on the vault's column policy: a vault issuing
    random tokens per occurrence breaks the join-key property this pipeline
    relies on. Confirm that before relying on cross-document resolution.
    """

    name = "skyflow"

    def __init__(self):
        self.url = os.environ.get("SKYFLOW_VAULT_URL", "").rstrip("/")
        self.vault_id = os.environ.get("SKYFLOW_VAULT_ID", "")
        self.token = os.environ.get("SKYFLOW_BEARER_TOKEN", "")
        self.table = os.environ.get("SKYFLOW_TABLE", "pii")
        self.column = os.environ.get("SKYFLOW_COLUMN", "value")
        missing = [n for n, v in (("SKYFLOW_VAULT_URL", self.url),
                                  ("SKYFLOW_VAULT_ID", self.vault_id),
                                  ("SKYFLOW_BEARER_TOKEN", self.token)) if not v]
        if missing:
            raise RuntimeError(f"skyflow vault needs: {', '.join(missing)}")
        self._cache: dict[str, str] = {}

    def _request(self, path: str, payload: dict) -> dict:
        import urllib.request
        req = urllib.request.Request(
            f"{self.url}/v1/vaults/{self.vault_id}/{path}",
            data=json.dumps(payload).encode(),
            headers={"Content-Type": "application/json",
                     "Authorization": f"Bearer {self.token}"},
        )
        import urllib.error
        try:
            return json.load(__import__("urllib.request", fromlist=["x"]).urlopen(req, timeout=60))
        except urllib.error.HTTPError as e:
            raise RuntimeError(f"skyflow {path} failed: {e.code} {e.read()[:200]!r}") from e

    def tokenise(self, kind: str, value: str) -> str:
        if value in self._cache:
            return self._cache[value]
        res = self._request(self.table, {"records": [{"fields": {self.column: value}}],
                                         "tokenization": True})
        try:
            token = res["records"][0]["tokens"][self.column]
        except (KeyError, IndexError) as e:
            raise RuntimeError(f"unexpected skyflow response shape: {res}") from e
        self._cache[value] = token
        return token

    def detokenise(self, token: str) -> str | None:
        res = self._request("detokenize", {"detokenizationParameters": [{"token": token}]})
        try:
            return res["records"][0]["value"]
        except (KeyError, IndexError):
            return None

    def save(self) -> None:
        """Nothing to persist: custody is the vendor's."""


class DatabunkerVault:
    """Databunker, self-hosted and open source.

    The middle option: real custody, in your own infrastructure, with no extra
    processor on the data map. Requires DATABUNKER_URL and DATABUNKER_TOKEN.
    """

    name = "databunker"

    def __init__(self):
        self.url = os.environ.get("DATABUNKER_URL", "").rstrip("/")
        self.token = os.environ.get("DATABUNKER_TOKEN", "")
        if not self.url or not self.token:
            raise RuntimeError("databunker vault needs DATABUNKER_URL and DATABUNKER_TOKEN")
        self._cache: dict[str, str] = {}

    def _post(self, path: str, payload: dict) -> dict:
        import urllib.request
        req = urllib.request.Request(
            f"{self.url}/v1/{path}", data=json.dumps(payload).encode(),
            headers={"Content-Type": "application/json", "X-Bunker-Token": self.token})
        return json.load(urllib.request.urlopen(req, timeout=60))

    def tokenise(self, kind: str, value: str) -> str:
        if value in self._cache:
            return self._cache[value]
        res = self._post("token", {"tokentype": kind.lower(), "record": value})
        token = res.get("token") or res.get("data", {}).get("token", "")
        if not token:
            raise RuntimeError(f"unexpected databunker response: {res}")
        self._cache[value] = token
        return token

    def detokenise(self, token: str) -> str | None:
        try:
            return self._post("token/get", {"token": token}).get("record")
        except Exception:
            return None

    def save(self) -> None:
        """Nothing to persist: custody is the server's."""


# --------------------------------------------------------------------------
# Composition
# --------------------------------------------------------------------------

class Tokeniser:
    """A detector plus a vault. This is the only type the pipeline knows about."""

    def __init__(self, detector: Detector, vault: VaultBackend):
        self.detector = detector
        self.vault = vault

    @property
    def description(self) -> str:
        return f"{self.detector.name} detection + {self.vault.name} custody"

    def tokenise(self, text: str) -> tuple[str, int]:
        """Replace every detected span with a stable token. Returns (text, count)."""
        spans = self.detector.detect(text)  # reverse order, so offsets stay valid
        for s in spans:
            token = self.vault.tokenise(s.kind, s.text)
            text = text[: s.start] + token + text[s.end:]
        return text, len(spans)

    def detokenise(self, text: str) -> str:
        """Reverse. In production this is entitlement-gated and audited."""
        for token in sorted(set(re.findall(r"TOK_[A-Z_]+_[0-9a-f]{6,}|tok_[\w-]{8,}", text)),
                            key=len, reverse=True):
            value = self.vault.detokenise(token)
            if value is not None:
                text = text.replace(token, value)
        return text

    def save(self) -> None:
        self.vault.save()


def build(detector: str | None = None, vault: str | None = None) -> Tokeniser:
    """Construct from configuration, falling back loudly rather than silently.

    A tokeniser that silently degrades to weaker detection is worse than one
    that fails, because the pipeline keeps running and the output looks fine.
    """
    # Default to the strongest detection available. `both` already degrades to
    # patterns with a warning when Presidio is absent, so defaulting to `regex`
    # bought nothing and cost real coverage: a run inheriting the default found
    # 1 sensitive value where the composite found 24, with nothing in the
    # output to indicate that detection had been weakened.
    d_name = (detector or os.environ.get("ONTO_DETECTOR", "both")).lower()
    v_name = (vault or os.environ.get("ONTO_VAULT", "local")).lower()

    def _composite():
        """Presidio plus patterns, degrading loudly if Presidio is absent.

        For the composite the run is still worth completing without Presidio,
        but silence would be dishonest: the log has to say detection was
        weakened, because the output looks identical either way. An explicit
        ONTO_DETECTOR=presidio still fails hard.
        """
        try:
            return CompositeDetector(PresidioDetector(), RegexDetector())
        except RuntimeError as e:
            import sys as _sys
            print(f"WARNING: Presidio unavailable, falling back to patterns only. "
                  f"Detection is materially weaker. ({e})", file=_sys.stderr)
            return RegexDetector()

    detectors = {"regex": RegexDetector, "presidio": PresidioDetector, "both": _composite}
    vaults = {"local": LocalVault, "skyflow": SkyflowVault, "databunker": DatabunkerVault}

    if d_name not in detectors:
        raise SystemExit(f"unknown ONTO_DETECTOR={d_name}. options: {', '.join(detectors)}")
    if v_name not in vaults:
        raise SystemExit(f"unknown ONTO_VAULT={v_name}. options: {', '.join(vaults)}")

    return Tokeniser(detectors[d_name](), vaults[v_name]())


if __name__ == "__main__":
    import sys

    piped = "" if sys.stdin.isatty() else sys.stdin.read()
    sample = piped.strip() or (
        "Dr Alice Morgan (alice.morgan@example.com, +44 20 7946 0958) reviewed "
        "batch DOC-20481 on 3 March. Contact Prof Brian Hale for the audit."
    )
    t = build()
    print(f"tokeniser: {t.description}\n")
    out, n = t.tokenise(sample)
    print(f"--- tokenised ({n} spans) ---\n{out}\n")
    print(f"--- detokenised (round trip) ---\n{t.detokenise(out)}\n")
    a, _ = t.tokenise("contact alice.morgan@example.com")
    b, _ = t.tokenise("write to alice.morgan@example.com")
    tok_a = re.search(r"TOK_\w+", a)
    tok_b = re.search(r"TOK_\w+", b)
    same = tok_a and tok_b and tok_a.group(0) == tok_b.group(0)
    print(f"determinism (same value -> same token across documents): {'YES' if same else 'NO'}")
    t.save()
