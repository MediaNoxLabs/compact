# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#  	http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

"""Build a portable, reproducible archive of the static comparison site."""

from hashlib import sha256
from pathlib import Path
from shutil import copyfile
from zipfile import ZIP_DEFLATED, ZipFile, ZipInfo


SITE = Path(__file__).resolve().parent
REPOSITORY = SITE.parents[1]
DIST = SITE / "dist"
ARCHIVE = REPOSITORY / "artifacts" / "compact-rust-atlas-m1.zip"
ASSETS = ("index.html", "styles.css", "app.js")


def build() -> None:
    html = (SITE / "index.html").read_text(encoding="utf-8")
    if './styles.css' not in html or './app.js' not in html:
        raise SystemExit("The HTML must reference both relative assets")
    for marker in ("127.0.0.1", "localhost", "file://", "/Users/"):
        if marker in html:
            raise SystemExit(f"Public HTML contains a local reference: {marker}")

    DIST.mkdir(exist_ok=True)
    unexpected = sorted(path.name for path in DIST.iterdir() if path.name not in ASSETS)
    if unexpected:
        raise SystemExit(f"Clean dist/ before packaging unexpected files: {unexpected}")

    ARCHIVE.parent.mkdir(exist_ok=True)
    with ZipFile(ARCHIVE, "w") as bundle:
        for name in ASSETS:
            source = SITE / name
            if not source.is_file():
                raise SystemExit(f"Missing site asset: {name}")
            copyfile(source, DIST / name)
            entry = ZipInfo(name, date_time=(1980, 1, 1, 0, 0, 0))
            entry.compress_type = ZIP_DEFLATED
            entry.external_attr = 0o100644 << 16
            bundle.writestr(entry, source.read_bytes())

    digest = sha256(ARCHIVE.read_bytes()).hexdigest()
    checksum = ARCHIVE.with_suffix(ARCHIVE.suffix + ".sha256")
    checksum.write_text(f"{digest}  {ARCHIVE.name}\n", encoding="ascii")
    print(f"Publish directory: {DIST}")
    print(f"Publication archive: {ARCHIVE}")
    print(f"SHA-256: {digest}")


if __name__ == "__main__":
    build()
