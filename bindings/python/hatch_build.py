"""builds the C library and puts it in the wheel, next to the license texts it ships under"""

import os
import subprocess
import sys
from pathlib import Path

from hatchling.builders.hooks.plugin.interface import BuildHookInterface

_LIBRARY = {"win32": "anybox.dll", "darwin": "libanybox.dylib"}.get(sys.platform, "libanybox.so")


class CustomBuildHook(BuildHookInterface):
    def initialize(self, version, build_data):
        if self.target_name != "wheel":
            # an sdist would install without the library and fail on import, so there isn't one
            raise RuntimeError("anybox-rs only ships wheels, build one with --wheel")
        from packaging.tags import sys_tags

        build_data["pure_python"] = False
        build_data["tag"] = f"py3-none-{next(iter(sys_tags())).platform}"
        workspace = Path(self.root).parents[1]
        # rust-toolchain.toml pins the toolchain, rustup installs it on the first cargo call
        if not os.environ.get("ANYBOX_SKIP_CARGO"):
            subprocess.run(
                ["cargo", "build", "-p", "anybox-capi", "--release", "--locked"], cwd=workspace, check=True
            )
        target = workspace / os.environ.get("CARGO_TARGET_DIR", "target")
        library = target / "release" / _LIBRARY
        if not library.exists():
            raise FileNotFoundError(f"there's no {library}, build anybox-capi with --release first")
        build_data["force_include"][str(library)] = f"anybox/{_LIBRARY}"
        notices = [workspace / "LICENSE", workspace / "NOTICE"]
        # the release workflow makes this one with cargo-about, a local build can go without it but a wheel that
        # ships (cibuildwheel sets CIBUILDWHEEL) can't
        third_party = target / "THIRD-PARTY-LICENSES.txt"
        if third_party.exists():
            notices.append(third_party)
        elif os.environ.get("CIBUILDWHEEL"):
            raise FileNotFoundError(f"there's no {third_party}, make it with cargo about first")
        for notice in notices:
            build_data["force_include"][str(notice)] = f"anybox/{notice.name}"
