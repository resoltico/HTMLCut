# SPDX-License-Identifier: MPL-2.0
"""Resolve Git Bash before the Windows WSL launcher for native Python callers."""
import os
from pathlib import Path


def bash_program():
    for key in ("ProgramW6432", "ProgramFiles", "ProgramFiles(x86)"):
        if root := os.environ.get(key):
            candidate = Path(root) / "Git/bin/bash.exe"
            if candidate.is_file():
                return str(candidate)
    return "bash"
