"""Cold starts must not race on the engine binary.

    python3 -m unittest web/try/test_engine_install.py

The function copies the bundled binary into the scratch directory before it
can run it. Concurrent cold-start invocations used to copy and chmod onto the
same final path, and the page answered 500 with `[Errno 26] Text file busy`.
These tests reproduce the race with many threads on one fake bundle. The first
two run anywhere; the third needs Linux, because only Linux refuses to open a
running executable for writing, and is skipped elsewhere with a reason.
"""
from __future__ import annotations

import os
import shutil
import subprocess
import sys
import tempfile
import threading
import unittest
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parent / "api"))
import index  # noqa: E402


def spawn(cmd: list[str]) -> subprocess.CompletedProcess:
    # The page spawns through `_run`, which keeps a fork out of an install's
    # write window. Use it when it exists so the test exercises what ships.
    run = getattr(index, "_run", None)
    if run is not None:
        return run(cmd, timeout=30)
    return subprocess.run(cmd, capture_output=True, text=True, timeout=30)


class EngineInstall(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp(prefix="oo-try-test-"))
        self.bundle = self.tmp / "bundle"
        (self.bundle / "_bin").mkdir(parents=True)
        self.scratch = self.tmp / "scratch"
        self.scratch.mkdir()
        os.environ.pop("OO_BIN", None)
        self.patches = [
            mock.patch.object(index, "HERE", self.bundle),
            mock.patch("tempfile.gettempdir", return_value=str(self.scratch)),
        ]
        for p in self.patches:
            p.start()

    def tearDown(self):
        for p in self.patches:
            p.stop()
        shutil.rmtree(self.tmp, ignore_errors=True)

    def fake_engine(self, marker: str, pad_lines: int = 60000) -> None:
        # A shell script whose only output is its LAST line, padded so a copy
        # takes long enough to be caught half-written. A truncated copy runs
        # and prints nothing; a copy without its mode bit cannot run at all.
        body = "#!/bin/sh\n" + ("# padding padding padding padding\n" * pad_lines) + f"echo {marker}\n"
        (self.bundle / "_bin" / "open-ontologies").write_text(body)

    def test_concurrent_cold_starts_only_ever_run_a_complete_binary(self):
        self.fake_engine("ENGINE_OK")
        n = 24
        barrier = threading.Barrier(n)
        results: list[object] = [None] * n

        def cold_start(i: int) -> None:
            barrier.wait()
            try:
                proc = spawn([str(index.engine())])
                results[i] = (proc.returncode, proc.stdout.strip(), proc.stderr.strip()[-200:])
            except Exception as e:  # noqa: BLE001
                results[i] = e

        threads = [threading.Thread(target=cold_start, args=(i,)) for i in range(n)]
        for t in threads:
            t.start()
        for t in threads:
            t.join()
        bad = [r for r in results if r != (0, "ENGINE_OK", "")]
        self.assertEqual(bad, [], f"{len(bad)} of {n} concurrent cold starts did not run the whole binary")

    def test_a_present_binary_is_not_rewritten_and_no_partial_copies_are_left(self):
        self.fake_engine("ENGINE_OK", pad_lines=10)
        first = index.engine()
        inode = first.stat().st_ino
        again = index.engine()
        self.assertEqual(first, again)
        self.assertEqual(again.stat().st_ino, inode, "an installed binary must not be rewritten")
        leftovers = [p.name for p in first.parent.iterdir() if p.name != first.name]
        self.assertEqual(leftovers, [], "no temporary copy may be left behind")
        self.assertEqual(spawn([str(again)]).stdout.strip(), "ENGINE_OK")

    @unittest.skipUnless(sys.platform.startswith("linux"), "ETXTBSY is a Linux error; macOS lets a running binary be written")
    def test_a_new_binary_can_replace_one_that_is_running(self):
        # The production failure, exactly: a real executable is running from
        # the installed path while another invocation installs a different
        # one. Writing in place fails with Errno 26 on Linux.
        sleep, true = shutil.which("sleep"), shutil.which("true")
        self.assertTrue(sleep and true)
        shutil.copyfile(sleep, self.bundle / "_bin" / "open-ontologies")
        running = subprocess.Popen([str(index.engine()), "3"])
        try:
            shutil.copyfile(true, self.bundle / "_bin" / "open-ontologies")
            if os.path.getsize(sleep) == os.path.getsize(true):
                self.skipTest("sleep and true are the same size here, so no reinstall is triggered")
            replaced = index.engine()
            self.assertEqual(spawn([str(replaced)]).returncode, 0)
        finally:
            running.kill()
            running.wait()


if __name__ == "__main__":
    unittest.main()
