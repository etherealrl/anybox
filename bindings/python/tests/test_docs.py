"""the Python examples in the docs, run over the mock, so one that stopped working fails here"""

import contextlib
import re
import threading
from pathlib import Path

import pytest

import anybox

_WORKSPACE = Path(__file__).resolve().parents[3]
_DOCS = ["docs/bindings.md", "bindings/python/README.md"]


def _examples() -> list[tuple[str, str]]:
    found = []
    for doc in _DOCS:
        text = (_WORKSPACE / doc).read_text(encoding="utf-8")
        for number, code in enumerate(re.findall(r"^```python\n(.*?)^```", text, re.MULTILINE | re.DOTALL), 1):
            found.append((f"{doc} {number}", code))
    return found


@pytest.mark.parametrize("code", [code for _, code in _examples()], ids=[name for name, _ in _examples()])
def test_the_example_runs(code, monkeypatch):
    device, box = anybox.mock_medius(anybox.ClickTiming.fixed(0.001), keyboard=True)
    monkeypatch.setattr(anybox, "connect", lambda: device)
    # a loop over the events ends once the device is closed, which nothing in the example does
    closers = []
    events = anybox.Device.events

    def events_then_close(self, *args, **kwargs):
        receiver = events(self, *args, **kwargs)
        closers.append(threading.Timer(0.2, self.close))
        closers[-1].start()
        return receiver

    monkeypatch.setattr(anybox.Device, "events", events_then_close)
    try:
        exec(compile(code, "example", "exec"), {})
    finally:
        for closer in closers:
            closer.cancel()
        # the example's `with` already closed it, this is for one that failed before getting there
        with contextlib.suppress(anybox.ClosedError):
            device.close()
        box.close()
        if hasattr(anybox.Device, "type_text"):
            del anybox.Device.type_text


def test_every_doc_has_its_examples():
    assert len(_examples()) == 5
