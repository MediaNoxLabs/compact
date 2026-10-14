"""Run dependency-free structural checks over the static storybook."""

from html.parser import HTMLParser
from pathlib import Path
from urllib.parse import urlparse


SITE = Path(__file__).resolve().parent
ASSETS = ("styles.css", "data.js", "app.js")
REQUIRED_COMMITS = (
    "eb72a5ab6085bf9b8564cc80c98c973e04825c2d",
    "589c92ef961ce89ac730140e81b76d4c08222a70",
    "03c03a39a2d39c43c91143b9526ca1c2ffae706c",
    "3799746acec6e5898e5e7b121621045d8dac925a",
    "ee768a60706607580d4ff8b3c949f22c882501b0",
    "e68078846260e4c6b2d18f40ec9ec1a2571cae33",
)


class StoryParser(HTMLParser):
    def __init__(self) -> None:
        super().__init__()
        self.ids: set[str] = set()
        self.links: list[str] = []

    def handle_starttag(self, _tag: str, attrs: list[tuple[str, str | None]]) -> None:
        values = dict(attrs)
        if values.get("id"):
            self.ids.add(values["id"] or "")
        if values.get("href"):
            self.links.append(values["href"] or "")


def check() -> None:
    html = (SITE / "index.html").read_text(encoding="utf-8")
    combined = html + "\n" + "\n".join((SITE / name).read_text(encoding="utf-8") for name in ASSETS)
    forbidden = ("file://", "/Users/", "127.0.0.1", "localhost")
    for marker in forbidden:
        if marker in combined:
            raise SystemExit(f"Machine-local or insecure reference found: {marker}")
    for commit in REQUIRED_COMMITS:
        if commit not in html:
            raise SystemExit(f"Required milestone commit link missing: {commit}")
    for asset in ASSETS:
        if not (SITE / asset).is_file():
            raise SystemExit(f"Missing asset: {asset}")

    parser = StoryParser()
    parser.feed(html)
    missing = sorted(link for link in parser.links if link.startswith("#") and link[1:] not in parser.ids)
    if missing:
        raise SystemExit(f"Broken fragment links: {', '.join(missing)}")
    unsafe = sorted(link for link in parser.links if urlparse(link).scheme not in ("", "https", "data"))
    if unsafe:
        raise SystemExit(f"Unexpected URL schemes: {', '.join(unsafe)}")
    print(f"OK: {len(parser.ids)} ids, {len(parser.links)} links, {len(ASSETS)} assets")


if __name__ == "__main__":
    check()
