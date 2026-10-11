"""The bridge to the verified checker, and the vocabulary of what a run earned.

This module states no verdict of its own. On acceptance it copies the checker's
words out of its stdout verbatim; on anything else it says, in its own words,
that nothing was checked. The two verdicts the checker can print are

* the absolute one, for a certificate over the BUILT-IN table, which
  `OOCert.Builtin.asHorn_sound` discharges against the semantics: every
  conclusion is true in every model of the asserted graph;
* the relativised one, `entailed_under_supplied_rules`, for a certificate over
  ANY other table, including the built-ins plus one extra rule: every conclusion
  is true in every model of the asserted graph THAT ALSO SATISFIES those rules.
  The rules are assumed and never checked, so a rule reading "every supplier is
  compliant" produces certificates that check green for ever.

Which of the two a run earned is decided inside `lean/HMain.lean`, by a full
ordered equality between the table it was handed and the built-in one. Deciding
it a second time here would make Python a second place where the two can be
confused, which is exactly the hazard decision 0003 exists to prevent, so this
module does not compare the tables, does not default the verdict, and does not
contain either verdict word as an executable string. `verdict` is only ever
assigned from the payload, and the rejection JSON has no `verdict` key at all,
so `payload.get("verdict", ...)` is not a hypothetical mistake: it is the
natural way to write that line and it is the exact shape of the laundering
failure.

The checker is an OPTIONAL EXTERNAL BINARY. It is built by `lake` from `lean/`,
it is deliberately not a Python dependency of any kind, not even an extra, and
this package is fully functional without it. What it must never do is report an
unchecked result as checked, so every outcome below carries `checked` and a
`status` word, and four of the six say plainly that nothing was proved.
"""

from __future__ import annotations

import json
import os
import shutil
import subprocess
from pathlib import Path

#: The theorem `HMain.lean` names when the table it was handed is the built-in
#: one. Used only to notice an impossible disagreement with the digests and say
#: so; it never overrides the checker and never synthesises a verdict.
_BUILTIN_THEOREM = "OOCert.entails_of_builtin_horn"

_INSTALL = (
    "Build it with `cd <repo>/lean && lake build` (it is in defaultTargets, so a bare "
    "`lake build` is enough), or set {env} to the binary"
)

_UNPROVED = (
    "Nothing here has been proved; these are the claims of an untrusted engine"
)


class CheckerUnavailable(RuntimeError):
    """The verified checker was not found. Reported, never worked around."""

    def __init__(self, binary: str, looked_in: list[str], install_line: str):
        self.binary = binary
        self.looked_in = looked_in
        self.install_line = install_line
        where = (
            f"Looked in {', '.join(looked_in)} and on PATH"
            if looked_in
            else "Looked on PATH, and nowhere else was named: there is no repository "
            "above this package, which is normal for a pip install"
        )
        super().__init__(
            f"{binary}, the verified checker, was not found. {install_line}. {where}"
        )


class CertificateUnreadable(RuntimeError):
    """The checker could not read a certificate this process had just written.

    Exit 2 is not a rejection. A run that wrote its own three files and is then
    told they cannot be read or parsed has an emitter bug, and folding that into
    the missing-binary branch is how such a bug gets mistaken for an uninstalled
    checker and never investigated.
    """


def _env_var(name: str) -> str:
    """`oo-horn` -> `OO_HORN`. The repo spells long-form `OPEN_ONTOLOGIES_*` for
    configuration and short `OO_*` for pointing at Lean-side artefacts
    (`OO_FOLMODEL`, `OO_DL_MODEL_DIR`), and this is one of those."""
    return name.upper().replace("-", "_")


def _repo_candidate(name: str) -> Path | None:
    """`<repo>/lean/.lake/build/bin/<name>`, if this file sits inside a checkout.

    A pip-installed wheel has no repository above it, so finding nothing here is
    normal and must not be an error.
    """
    for parent in Path(__file__).resolve().parents:
        if (parent / "lean" / "lakefile.toml").is_file():
            return parent / "lean" / ".lake" / "build" / "bin" / name
    return None


def find_checker(name: str = "oo-horn", explicit: str | Path | None = None) -> Path:
    """Locate the checker: explicit, then the environment, then the repository
    build directory, then PATH.

    That order is the Rust precedent (`src/fol_solve.rs:278-306`), and the repo
    probe comes before PATH deliberately: a developer working in the checkout has
    just run `lake build`, the repo binary is the one they mean, and a stale
    `oo-horn` on PATH silently shadowing a freshly built one is a bad half-hour.

    The docstring of the Rust original is the reason this raises rather than
    degrading: "Its absence is reported rather than worked around. A pipeline
    that quietly dropped to an oracle verdict because the checker was not built
    would make every run look like a solver limitation."
    """
    looked: list[str] = []
    env = _env_var(name)
    if explicit is not None:
        # An explicit path does NOT fall through, matching the Rust precedent: a
        # caller who named a binary meant that binary, and quietly running a
        # different one found on PATH would make a report about the wrong
        # checker. The environment variable does fall through, because it is a
        # site-wide default rather than a per-call instruction.
        p = Path(explicit)
        if p.is_file() and os.access(p, os.X_OK):
            # Keep a verified relative path from becoming a bare PATH lookup.
            # Preserve symlink identity; only anchor it to the current directory.
            return p.absolute()
        raise CheckerUnavailable(
            name,
            [str(p)],
            f"no executable {name} at {p}, and a path given explicitly is not replaced "
            f"by a search",
        )
    from_env = os.environ.get(env)
    if from_env:
        p = Path(from_env)
        looked.append(f"{env}={from_env}")
        if p.is_file() and os.access(p, os.X_OK):
            # Keep a verified relative path from becoming a bare PATH lookup.
            # Preserve symlink identity; only anchor it to the current directory.
            return p.absolute()
    repo = _repo_candidate(name)
    if repo is not None:
        looked.append(str(repo))
        if repo.is_file() and os.access(repo, os.X_OK):
            return repo
    on_path = shutil.which(name)
    if on_path:
        return Path(on_path)
    raise CheckerUnavailable(name, looked, _INSTALL.format(env=env))


def _check_with(certificate_dir: Path, name: str) -> str:
    return (
        f"{name} check {certificate_dir / 'rules.tsv'} "
        f"{certificate_dir / 'asserted.tsv'} {certificate_dir / 'horn.tsv'}"
    )


def _unchecked(status: str, certificate_dir: Path, name: str, **extra) -> dict:
    """Every outcome in which the checker did not pronounce. `verdict` is present
    and None rather than absent, so a caller reading `report["check"]["verdict"]`
    cannot get a KeyError and reach for a default."""
    out = {
        "status": status,
        "checked": False,
        "verdict": None,
        "pronounced_by": None,
        "check_with": _check_with(certificate_dir, name),
        "warning": None,
    }
    out.update(extra)
    return out


def check_horn(
    certificate_dir: str | Path,
    *,
    checker: str | Path | None = None,
    timeout: float = 120.0,
    name: str = "oo-horn",
) -> dict:
    """Run the Lean checker over a certificate directory and report what it said.

    Six outcomes, one `status` word each: `accepted`, `rejected`, `unreadable`,
    `unchecked_no_checker`, `unchecked_timeout`, `unchecked_error`. Only the first
    two have `checked` true, and only the first carries a verdict, copied from the
    checker's stdout and never computed here.

    A read or parse error is NOT a rejection: exit 1 means a step was rejected,
    exit 2 means the checker could not read what it was given. This is a library
    primitive over a directory it did not necessarily write, so it returns that as
    `unreadable` in the package's `ValidationResult(ok=False, error=...)` idiom;
    `OntologyEngine.reason_horn`, which did write the files, raises on it instead.
    """
    certificate_dir = Path(certificate_dir)
    try:
        binary = find_checker(name, checker)
    except CheckerUnavailable as exc:
        return _unchecked(
            "unchecked_no_checker",
            certificate_dir,
            name,
            missing=exc.binary,
            looked_in=exc.looked_in,
            how_to_get_it=exc.install_line,
            warning=(
                f"the certificate was written and NOT checked: {exc}. {_UNPROVED}"
            ),
        )
    argv = [
        str(binary),
        "check",
        str(certificate_dir / "rules.tsv"),
        str(certificate_dir / "asserted.tsv"),
        str(certificate_dir / "horn.tsv"),
    ]
    try:
        # Always a timeout, never shell=True. `oo-horn` writes one JSON line so a
        # pipe cannot fill, but an MCP server that hangs for ever on a missing
        # timeout is a worse bug than the one it avoids.
        proc = subprocess.run(argv, capture_output=True, text=True, timeout=timeout)
    except subprocess.TimeoutExpired:
        return _unchecked(
            "unchecked_timeout",
            certificate_dir,
            name,
            timeout_seconds=timeout,
            error=f"{binary} did not finish within {timeout} seconds",
            warning=f"the certificate was written and NOT checked. {_UNPROVED}",
        )
    except OSError as exc:  # a binary that will not execute at all
        return _unchecked(
            "unchecked_error",
            certificate_dir,
            name,
            error=f"cannot run {binary}: {exc}",
            warning=f"the certificate was written and NOT checked. {_UNPROVED}",
        )

    if proc.returncode == 2:
        # Exit 2 writes to STDERR and nothing to stdout. A bridge that captured
        # only stdout would get "" here and a json.loads failure that reads like a
        # checker bug.
        return _unchecked(
            "unreadable",
            certificate_dir,
            name,
            error=proc.stderr.strip() or proc.stdout.strip(),
            means=(
                "a read or parse error is not a rejection: exit 1 means a step was "
                "rejected, exit 2 means the checker could not read what it was given"
            ),
            warning=f"the certificate was written and NOT checked. {_UNPROVED}",
        )

    try:
        payload = json.loads(proc.stdout)
    except (ValueError, TypeError):
        return _unchecked(
            "unchecked_error",
            certificate_dir,
            name,
            error=(
                f"{binary} exited {proc.returncode} and its output is not JSON: "
                f"{proc.stdout.strip()[:400]!r} / {proc.stderr.strip()[:400]!r}"
            ),
            warning=f"the certificate was written and NOT checked. {_UNPROVED}",
        )

    if proc.returncode == 1 and payload.get("ok") is False:
        return {
            "status": "rejected",
            "checked": True,
            "verdict": None,
            "pronounced_by": str(binary),
            "checker_report": payload,
            "means": (
                "the Lean checker rejected at least one step of this certificate. It "
                "establishes nothing"
            ),
            "files": _check_with(certificate_dir, name),
            "check_with": _check_with(certificate_dir, name),
        }

    if proc.returncode != 0 or payload.get("ok") is not True:
        return _unchecked(
            "unchecked_error",
            certificate_dir,
            name,
            error=(
                f"{binary} exited {proc.returncode} with {proc.stdout.strip()[:400]!r}, "
                f"which is neither an acceptance nor a rejection"
            ),
            warning=f"the certificate was written and NOT checked. {_UNPROVED}",
        )

    try:
        # No default, no fallback, no conditional. If a payload the checker called
        # ok is missing one of these, that is an unknown checker and not an
        # acceptance this bridge may summarise.
        verdict = payload["verdict"]
        theorem = payload["theorem"]
        means = payload["means"]
        rules_digest = payload["rules_digest"]
        builtin_digest = payload["builtin_rules_digest"]
        derivations = payload["derivations"]
    except KeyError as exc:
        return _unchecked(
            "unchecked_error",
            certificate_dir,
            name,
            error=(
                f"{binary} accepted the certificate but its report has no {exc} key, so "
                f"this is not a checker this bridge understands"
            ),
            warning=f"the certificate was written and NOT checked. {_UNPROVED}",
        )

    digests_agree = rules_digest == builtin_digest
    out = {
        "status": "accepted",
        "checked": True,
        "verdict": verdict,
        "theorem": theorem,
        "means": means,
        "pronounced_by": str(binary),
        "rules_digest": rules_digest,
        "builtin_rules_digest": builtin_digest,
        "digests_agree": digests_agree,
        "derivations": derivations,
        # An empty horn.tsv is accepted with zero derivations, and the checker is
        # right: vacuously every step in an empty list holds. The report would be
        # what was wrong, so a verdict is never rendered here without the count
        # beside it.
        "vacuous": derivations == 0,
        "check_with": _check_with(certificate_dir, name),
        "checker_report": payload,
    }
    if theorem == _BUILTIN_THEOREM and not digests_agree:
        # Cannot happen: the checker derives both from the same comparison. If it
        # ever does, the anomaly is REPORTED and the checker is still not
        # overridden, because a gate that can only weaken is still a second place
        # where the two warrants are decided.
        out["consistency_warning"] = (
            "the checker named the built-in theorem while reporting a rule digest that "
            "differs from the built-in table's. Do not rely on this report; rerun and "
            "raise it against lean/"
        )
    return out
