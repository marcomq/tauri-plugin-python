#!/usr/bin/env python3
"""Deployment-recipe smoke check for the plain-javascript example.

After `npm run tauri build`, this asserts that the Python sources were actually
copied into the produced app bundle as tauri resources. This is the single most
common "works in `tauri dev`, not in the production build" failure (see the
README's Deployment section), so we guard against it in CI.

It does NOT need a display or a real Python runtime - it only inspects what the
bundler emitted. Run from the example's `src-tauri` directory, or pass the
target dir as the first argument.
"""
import sys
from pathlib import Path


def main() -> int:
    target = Path(sys.argv[1]) if len(sys.argv) > 1 else Path("target")
    if not target.is_dir():
        print(f"FAIL: target dir not found: {target.resolve()}")
        return 1

    # tauri copies resources into the bundle next to the app. The exact layout
    # differs per OS/format:
    #   - macOS: *.app/Contents/Resources/ (the .app lives under */bundle/macos)
    #   - Linux AppImage: */bundle/appimage/*.AppDir/usr/... (unpacked tree)
    #   - Windows NSIS/MSI: resources are NOT left loose under bundle/ - they get
    #     packed *inside* the installer. The loose copy lives next to the binary
    #     at target/release/ instead.
    # So search the whole target tree for our resource. The original sources live
    # at the repo root (../src-python), not under target/, so a hit here always
    # means the bundler emitted it as a resource next to/inside the app.
    search_roots = list(target.glob("*/bundle")) + list(target.glob("bundle"))
    search_roots += list(target.glob("*/release")) + list(target.glob("release"))
    if not search_roots:
        print(f"FAIL: nothing built under {target.resolve()} - did `tauri build` run?")
        return 1

    hits = []
    for root in search_roots:
        for main_py in root.rglob("src-python/main.py"):
            # Be tolerant of casing/symlinks; just confirm it's a real file.
            if main_py.is_file():
                hits.append(main_py)

    if not hits:
        print("FAIL: no bundled 'src-python/main.py' found alongside the produced app.")
        print("      The Python sources were not shipped as tauri resources.")
        print("      Check `bundle.resources` in tauri.conf.json.")
        print(f"      Searched: {[str(r) for r in search_roots]}")
        return 1

    print("OK: Python sources are bundled as resources. Found:")
    for h in hits:
        print(f"  - {h}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
