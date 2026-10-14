"""Build a portable, deterministic archive of the Rust-AST storybook."""

from hashlib import sha256
from pathlib import Path
from shutil import copyfile, rmtree
from zipfile import ZIP_DEFLATED, ZipFile, ZipInfo


SITE = Path(__file__).resolve().parent
REPOSITORY = SITE.parents[1]
DIST = SITE / "dist"
PAGES = REPOSITORY / "docs"
ARCHIVE = REPOSITORY / "artifacts" / "compact-rust-ast-storybook.zip"
ASSETS = ("index.html", "styles.css", "data.js", "app.js")


def build() -> None:
    for name in ASSETS:
        if not (SITE / name).is_file():
            raise SystemExit(f"Missing site asset: {name}")

    if DIST.exists():
        rmtree(DIST)
    DIST.mkdir()
    PAGES.mkdir(exist_ok=True)
    unexpected = sorted(path.name for path in PAGES.iterdir() if path.name not in (*ASSETS, ".nojekyll"))
    if unexpected:
        raise SystemExit(f"Refusing to replace unexpected GitHub Pages files: {unexpected}")
    ARCHIVE.parent.mkdir(exist_ok=True)

    with ZipFile(ARCHIVE, "w") as bundle:
        for name in ASSETS:
            source = SITE / name
            copyfile(source, DIST / name)
            copyfile(source, PAGES / name)
            entry = ZipInfo(name, date_time=(1980, 1, 1, 0, 0, 0))
            entry.compress_type = ZIP_DEFLATED
            entry.external_attr = 0o100644 << 16
            bundle.writestr(entry, source.read_bytes())

    (PAGES / ".nojekyll").write_text("", encoding="ascii")

    digest = sha256(ARCHIVE.read_bytes()).hexdigest()
    checksum = ARCHIVE.with_suffix(ARCHIVE.suffix + ".sha256")
    checksum.write_text(f"{digest}  {ARCHIVE.name}\n", encoding="ascii")
    print(f"Publish directory: {DIST}")
    print(f"GitHub Pages directory: {PAGES}")
    print(f"Publication archive: {ARCHIVE}")
    print(f"SHA-256: {digest}")


if __name__ == "__main__":
    build()
