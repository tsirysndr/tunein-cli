#!/usr/bin/env python3
"""Regenerate committed module bindings without uploading the application tree."""
from pathlib import Path
import os
import shutil
import subprocess
import tempfile

here = Path(__file__).resolve().parent
root = here.parent
cli = os.environ.get("DAGGER_CLI_BIN", str(here / "bin/dagger"))
if not Path(cli).is_file() and not shutil.which(cli):
    raise SystemExit("Run .dagger/install-dagger.sh first, or set DAGGER_CLI_BIN")

with tempfile.TemporaryDirectory(prefix="tunein-cli-dagger-codegen-") as temporary:
    workspace = Path(temporary)
    shutil.copytree(here, workspace / ".dagger", ignore=shutil.ignore_patterns(
        "bin", "node_modules", ".spago", "output", "__pycache__"))
    shutil.copy2(root / "dagger.toml", workspace / "dagger.toml")
    if (root / "dagger.lock").exists():
        shutil.copy2(root / "dagger.lock", workspace / "dagger.lock")
    subprocess.run(["git", "init", "--quiet", str(workspace)], check=True)
    subprocess.run([cli, "generate", "-y"], cwd=workspace / ".dagger", check=True)
    # Publish only generated artifacts after the entire generation succeeds.
    # Handwritten src/, spago.yaml, and application files are never copied back.
    for module in ("ci",):
        generated = workspace / ".dagger" / module
        target = here / module
        shutil.rmtree(target / "dagger-sdk")
        shutil.copytree(generated / "dagger-sdk", target / "dagger-sdk")
        for filename in ("dagger-module.toml", ".gitignore", ".gitattributes"):
            if (generated / filename).exists():
                shutil.copy2(generated / filename, target / filename)
    if (workspace / "dagger.lock").exists():
        shutil.copy2(workspace / "dagger.lock", root / "dagger.lock")
print("Regenerated ci/ bindings")
