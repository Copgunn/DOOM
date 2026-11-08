#!/usr/bin/env python3
"""Check both frozen runner contracts with LF and Windows CRLF source copies."""
import copy
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import m3h9c_build as base
import m4a_build


class RunnerNewlines(unittest.TestCase):
    def test_checkout_newlines_preserve_both_frozen_contracts(self):
        root = base.ROOT
        runners = [base.M3H9C, m4a_build.M4A]
        expected = {r.name: json.loads(r.contract.read_text(encoding='utf-8')) for r in runners}
        paths = ({base.SOURCE / f for f in base.WHOLE + ['r_main.c']}
                 | set(base.SOURCE.glob('*.h')) | {base.STUBS}
                 | {r.driver for r in runners})
        for newline in [b'\n', b'\r\n']:
            with self.subTest(newline=newline), tempfile.TemporaryDirectory() as directory:
                scratch = Path(directory)
                for path in paths:
                    destination = scratch / path.relative_to(root)
                    destination.parent.mkdir(parents=True, exist_ok=True)
                    data = path.read_bytes().replace(b'\r\n', b'\n')
                    destination.write_bytes(data.replace(b'\n', newline))
                with patch.multiple(base, ROOT=scratch, SOURCE=scratch / 'linuxdoom-1.10',
                                    STUBS=scratch / base.STUBS.relative_to(root)):
                    for runner in runners:
                        with self.subTest(runner=runner.name):
                            local = copy.copy(runner)
                            local.driver = scratch / runner.driver.relative_to(root)
                            self.assertEqual(base.contract(local), expected[runner.name])


if __name__ == '__main__':
    unittest.main()
