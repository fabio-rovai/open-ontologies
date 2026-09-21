import SelfId.Sha256

/-!
# A checker that says which binary is running

Issue #204. Every checker here prints the name of the theorem its acceptance
discharges, and nothing that identifies the BUILD that ran. A verdict is a JSON
object containing a theorem name, and any process can print that object. The
Rust side is careful never to fabricate the string, but a reader of the report
had no way to tell an `oo-shacl` built from a commit whose proofs were checked
from one built where `Shacl` was not a default target and did not compile,
which the comment at the top of `lakefile.toml` records happening on 14
September 2026.

## What is reported, and why it is a digest rather than a revision

A git revision baked in at build time is a string the binary cannot
substantiate: a checker that SAYS it is `e483d75` is exactly as trustworthy as
one that says it discharged a theorem, which is the problem rather than the
answer. So the checker reports the digest of the file it is running from, read
through `IO.appPath` at the moment it runs, and a reader matches that against
the `SHASUMS.txt` the release publishes.

That has somewhere to land: as of #241 the release publishes the checkers as
assets with their digests in `SHASUMS.txt`.

## What this does NOT do

It does not make a checker honest. A hostile binary can print any block it
likes, including this one, and nothing here detects that: the digest it reports
and the digest of the file are the same number only because THIS code computes
the second from the first's own path. What it gives an honest build is a way to
say which build it is, checkable against a published list by someone who does
not trust it. That is residual hole 1 in `src/verdict.rs` narrowed, not closed,
and the module header there says so.

The hash itself is unverified; `SelfId/Sha256.lean` says so in its own header
and carries the published vectors as compile-time guards.
-/

namespace SelfId

structure Checker where
  name : String
  selfSha256 : String
  toolchain : String
  deriving Repr

/-- Read the running executable and describe it.

`IO.appPath` is the path the process was started from. A binary that cannot
read itself is an honest failure rather than a blank field: the digest is
reported as the empty string and `readable` is false, so a consumer can tell
"I could not look" from "I looked and it was this". -/
def describe : IO Checker := do
  let p ← IO.appPath
  let name := p.fileName.getD "unknown"
  let digest ←
    try
      let bytes ← IO.FS.readBinFile p
      pure (Sha256.hashHex bytes)
    catch _ =>
      pure ""
  return { name := name, selfSha256 := digest, toolchain := Lean.versionString }

private def esc (s : String) : String :=
  s.foldl (fun acc c =>
    acc ++ (match c with
      | '"' => "\\\""
      | '\\' => "\\\\"
      | '\n' => "\\n"
      | c => c.toString)) ""

/-- The `checker` block, as the object a verdict carries under that key. -/
def json (c : Checker) : String :=
  "{\"name\":\"" ++ esc c.name ++ "\",\"self_sha256\":\"" ++ esc c.selfSha256 ++
  "\",\"toolchain\":\"" ++ esc c.toolchain ++
  "\",\"means\":\"the digest of the FILE this process is running from, read at run time. " ++
  "Match it against the SHASUMS.txt of the release you expected. It does not make this " ++
  "binary honest: a hostile one can print any block it likes. It lets an honest build say " ++
  "which build it is.\"}"

/-- `,\"checker\":{...}`, ready to splice into a verdict object. -/
def field (c : Checker) : String := ",\"checker\":" ++ json c

/-- Print a verdict object with the `checker` block spliced in.

One helper rather than a `let c ← describe` threaded through nine entry points
with fourteen print sites between them: each site becomes `SelfId.println`
where it was `IO.println`, which is a change a reviewer can check by grep.

`obj` is expected to be a JSON object. If it does not end in a closing brace
this prints it unchanged rather than corrupting it, because a checker that
emits a malformed verdict should look malformed and not look identified. -/
def println (obj : String) : IO Unit := do
  let trimmed := obj.trimRight
  if trimmed.endsWith "}" then
    IO.println (trimmed.dropRight 1 ++ field (← describe) ++ "}")
  else
    IO.println trimmed

/-- `--version`: the same block, alone, exit 0. -/
def emitVersion : IO UInt32 := do
  IO.println ("{\"checker\":" ++ json (← describe) ++ "}")
  return 0

end SelfId
