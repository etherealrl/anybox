"""builds the capi with the mock and points the package at it before anything imports anybox"""

import os
import subprocess
import sys
from pathlib import Path

import pytest

_ROOT = Path(__file__).resolve().parents[1]
_WORKSPACE = _ROOT.parents[1]
_LIBRARY = {"win32": "anybox.dll", "darwin": "libanybox.dylib"}.get(sys.platform, "libanybox.so")

if "ANYBOX_LIB" not in os.environ:
    subprocess.run(
        ["cargo", "build", "-p", "anybox-capi", "--features", "mock", "--locked"], cwd=_WORKSPACE, check=True
    )
    # cargo takes a relative CARGO_TARGET_DIR from where it runs, the workspace
    target = _WORKSPACE / os.environ.get("CARGO_TARGET_DIR", "target")
    os.environ["ANYBOX_LIB"] = str(target / "debug" / _LIBRARY)
sys.path.insert(0, str(_ROOT))


@pytest.fixture
def workspace() -> Path:
    return _WORKSPACE


@pytest.fixture
def mock():
    import anybox

    device, box = anybox.mock_medius()
    yield device, box
    if not device._closed:
        device.close()
    box.close()
