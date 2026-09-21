import MatCert.All
import SelfId.All

/-!
# `oo-matcert`: check a numeric claim

    oo-matcert CERT.matcert

`matcert/1` is a line format, tab separated, integers only:

    matcert/1
    dims<TAB>m<TAB>p<TAB>n
    A<TAB>1 2 3
    B<TAB>4 5
    C<TAB>...

Exit 0 with a verdict, 1 when the claim does not check, 2 when the file cannot
be read or parsed. The same three codes every checker here uses, so a caller
that already handles one handles this.

Integers by construction. The parser refuses anything with a decimal point
rather than rounding it, because a certificate about floating-point numbers
checked as if they were reals is unsound in a way that never announces itself.
That path exists in Rust, says so, and does not print a theorem name.
-/

open Mat

def jsonStr (s : String) : String :=
  "\"" ++ (s.foldl (fun acc c =>
    acc ++ (match c with
      | '"' => "\\\"" | '\\' => "\\\\" | '\n' => "\\n" | c => c.toString)) "") ++ "\""

def emitError (reason : String) : IO UInt32 := do
  IO.println ("{\"ok\":false,\"error\":" ++ jsonStr reason ++ "}")
  return 2

/-- One integer, or nothing. A decimal point is refused by name. -/
def readInt (s : String) : Option Int :=
  let t := s.trim
  if t.isEmpty then none
  else if t.contains '.' then none
  else if t.startsWith "-" then (String.toNat? (String.mk (t.toList.drop 1))).map (fun n => -(Int.ofNat n))
  else (String.toNat? t).map Int.ofNat

def readRow (s : String) : Option (List Int) :=
  ((s.trim.splitOn " ").filter (fun w => !w.isEmpty)).foldr
    (fun w acc => match acc, readInt w with
      | some xs, some v => some (v :: xs)
      | _, _ => none)
    (some [])

structure Cert where
  m : Nat
  p : Nat
  n : Nat
  A : Matrix
  B : Matrix
  C : Matrix

def parse (text : String) : Except String Cert := do
  let lines := (text.splitOn "\n").filter (fun l => !l.trim.isEmpty)
  match lines with
  | [] => throw "the certificate is empty"
  | hdr :: rest =>
    if hdr.trim != "matcert/1" then
      throw s!"expected a matcert/1 header, found {hdr.trim}"
    else
      let mut dims : Option (Nat × Nat × Nat) := none
      let mut a : List (List Int) := []
      let mut b : List (List Int) := []
      let mut c : List (List Int) := []
      for line in rest do
        match line.splitOn "\t" with
        | ["dims", ms, ps, ns] =>
          match String.toNat? ms.trim, String.toNat? ps.trim, String.toNat? ns.trim with
          | some m, some p, some n => dims := some (m, p, n)
          | _, _, _ => throw s!"dims must be three naturals: {line}"
        | [tag, row] =>
          match readRow row with
          | none => throw s!"a row carries something that is not an integer: {line}. \
              Refused rather than rounded: a numeric certificate that quietly turned 1.5 into \
              1 would check a claim nobody made."
          | some r =>
            if tag == "A" then a := a ++ [r]
            else if tag == "B" then b := b ++ [r]
            else if tag == "C" then c := c ++ [r]
            else throw s!"unknown row tag {tag}"
        | _ => throw s!"unreadable line: {line}"
      match dims with
      | none => throw "the certificate states no dims line"
      | some (m, p, n) => return { m := m, p := p, n := n, A := a, B := b, C := c }

def main (args : List String) : IO UInt32 := do
  match args with
  | ["--version"] => SelfId.emitVersion
  | [path] =>
    let text ← try IO.FS.readFile path catch _ =>
      return ← emitError s!"could not read {path}"
    match parse text with
    | .error e => emitError e
    | .ok cert =>
      if check cert.A cert.B cert.C cert.m cert.p cert.n then
        SelfId.println ("{\"ok\":true,\"verdict\":\"product_checked\"," ++
          "\"m\":" ++ toString cert.m ++ ",\"p\":" ++ toString cert.p ++
          ",\"n\":" ++ toString cert.n ++
          ",\"theorem\":\"MatCert.mul_of_check\"," ++
          "\"means\":" ++ jsonStr
            "every entry of C within the stated shape is the sum over k of A[i][k]*B[k][j], \
             recomputed here from the definition. Integers only. This says nothing about any \
             floating-point computation that produced these numbers." ++ "}")
        return 0
      else
        SelfId.println ("{\"ok\":false,\"verdict\":\"product_not_checked\"," ++
          "\"means\":" ++ jsonStr
            "either the shapes disagree with the stated dims or some entry of C is not the \
             sum of products. No theorem is named, because none was discharged." ++ "}")
        return 1
  | _ =>
    IO.eprintln "usage: oo-matcert CERT.matcert"
    return 2
