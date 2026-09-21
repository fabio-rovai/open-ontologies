/-!
# SHA-256, in core Lean

A checker that reports the digest of the binary that is running needs a
SHA-256, and this repository takes no dependencies beyond the Lean kernel and
what is written in `lean/`. So it is written here.

**This is NOT a verified implementation and nothing pretends otherwise.** No
theorem in this repository says that `Sha256.hash` computes the function FIPS
180-4 defines. What stands behind it is the ordinary thing that stands behind
any hash implementation: it reproduces the published test vectors, and it
agrees with `sha256sum` on real files, both of which are checked by
`tests/checker_self_identification_test.rs`.

That is the honest level of assurance for this particular job. The digest
exists so a reader can match a running checker against the `SHASUMS.txt` a
release publishes. If this implementation were wrong it would disagree with
`sha256sum` and the match would fail loudly rather than succeed falsely, which
is the failure direction that costs nothing.

The implementation is the specification's own loop, over `ByteArray` rather
than lists, because the thing being hashed is a five-megabyte executable and a
cons list of bytes would not finish.
-/

namespace Sha256

/-- The 64 round constants: the first 32 bits of the fractional parts of the
cube roots of the first 64 primes. -/
def K : Array UInt32 := #[
  0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
  0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
  0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
  0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
  0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
  0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
  0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
  0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2]

/-- The initial hash value: the first 32 bits of the fractional parts of the
square roots of the first eight primes. -/
def H0 : Array UInt32 :=
  #[0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19]

@[inline] def rotr (x : UInt32) (n : UInt32) : UInt32 :=
  (x >>> n) ||| (x <<< (32 - n))

@[inline] def ch (x y z : UInt32) : UInt32 := (x &&& y) ^^^ ((~~~x) &&& z)
@[inline] def maj (x y z : UInt32) : UInt32 := (x &&& y) ^^^ (x &&& z) ^^^ (y &&& z)
@[inline] def bigS0 (x : UInt32) : UInt32 := rotr x 2 ^^^ rotr x 13 ^^^ rotr x 22
@[inline] def bigS1 (x : UInt32) : UInt32 := rotr x 6 ^^^ rotr x 11 ^^^ rotr x 25
@[inline] def smallS0 (x : UInt32) : UInt32 := rotr x 7 ^^^ rotr x 18 ^^^ (x >>> 3)
@[inline] def smallS1 (x : UInt32) : UInt32 := rotr x 17 ^^^ rotr x 19 ^^^ (x >>> 10)

/-- The big-endian 32-bit word at byte offset `i`. -/
@[inline] def wordAt (b : ByteArray) (i : Nat) : UInt32 :=
  (b[i]!.toUInt32 <<< 24) ||| (b[i+1]!.toUInt32 <<< 16)
    ||| (b[i+2]!.toUInt32 <<< 8) ||| b[i+3]!.toUInt32

/-- One 64-byte block, starting at `off`, folded into the running state. -/
def block (st : Array UInt32) (b : ByteArray) (off : Nat) : Array UInt32 := Id.run do
  -- The message schedule: sixteen words read from the block, forty-eight
  -- derived from them.
  let mut w : Array UInt32 := Array.mkEmpty 64
  for i in [0:16] do
    w := w.push (wordAt b (off + 4 * i))
  for i in [16:64] do
    let s0 := smallS0 w[i-15]!
    let s1 := smallS1 w[i-2]!
    w := w.push (w[i-16]! + s0 + w[i-7]! + s1)
  let mut a := st[0]!; let mut bb := st[1]!; let mut c := st[2]!; let mut d := st[3]!
  let mut e := st[4]!; let mut f := st[5]!; let mut g := st[6]!; let mut h := st[7]!
  for i in [0:64] do
    let t1 := h + bigS1 e + ch e f g + K[i]! + w[i]!
    let t2 := bigS0 a + maj a bb c
    h := g; g := f; f := e; e := d + t1
    d := c; c := bb; bb := a; a := t1 + t2
  #[st[0]! + a, st[1]! + bb, st[2]! + c, st[3]! + d,
    st[4]! + e, st[5]! + f, st[6]! + g, st[7]! + h]

/-- The padded message: the bytes, a `0x80`, zeroes to 56 mod 64, then the
length in BITS as a 64-bit big-endian integer. -/
def pad (b : ByteArray) : ByteArray := Id.run do
  let len := b.size
  let bits : UInt64 := (UInt64.ofNat len) * 8
  let mut out := b.push 0x80
  while out.size % 64 != 56 do
    out := out.push 0
  for i in [0:8] do
    out := out.push (((bits >>> (UInt64.ofNat (56 - 8 * i))) &&& 0xff).toUInt8)
  return out

/-- The digest, as 32 bytes. -/
def hash (b : ByteArray) : ByteArray := Id.run do
  let m := pad b
  let mut st := H0
  let mut off := 0
  while off < m.size do
    st := block st m off
    off := off + 64
  let mut out := ByteArray.empty
  for x in st do
    out := out.push ((x >>> 24) &&& 0xff).toUInt8
    out := out.push ((x >>> 16) &&& 0xff).toUInt8
    out := out.push ((x >>> 8) &&& 0xff).toUInt8
    out := out.push (x &&& 0xff).toUInt8
  return out

def hexDigit (n : UInt8) : Char :=
  if n < 10 then Char.ofNat (n.toNat + '0'.toNat) else Char.ofNat (n.toNat - 10 + 'a'.toNat)

/-- The digest as the lower-case hex a `SHASUMS.txt` line carries. -/
def hex (b : ByteArray) : String := Id.run do
  let mut s := ""
  for x in b do
    s := s.push (hexDigit (x >>> 4))
    s := s.push (hexDigit (x &&& 0x0f))
  return s

def hashHex (b : ByteArray) : String := hex (hash b)

end Sha256

/-! ## The published vectors, checked when this file compiles

FIPS 180-4's own examples, plus the empty string. `#guard` evaluates them at
elaboration time, so a build of this library that computes a different digest
does not produce a library. It is a compile-time check and not a theorem: it
says this implementation agrees with three known answers, which is what a hash
implementation can honestly claim without a proof about the standard.

`tests/checker_self_identification_test.rs` does the other half, comparing a
running checker's report of its own digest against `sha256sum` over the same
file, which is a three-megabyte input rather than three small ones. -/

#guard Sha256.hashHex "".toUTF8 ==
  "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
#guard Sha256.hashHex "abc".toUTF8 ==
  "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
#guard Sha256.hashHex "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq".toUTF8 ==
  "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
