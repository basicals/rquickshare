"""Own a temporary signed test channel while Node serves it through local workerd."""
import json
from contextlib import redirect_stdout
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[3] / "scripts/tests"))
from test_native_apt import NativeAptTests
from native_apt import PRODUCTION_URI

with redirect_stdout(sys.stderr):
    NativeAptTests.setUpClass()
try:
    print(json.dumps({"repository": str(NativeAptTests.signed), "production_uri": PRODUCTION_URI}), flush=True)
    command = sys.stdin.readline()
    if command:
        with redirect_stdout(sys.stderr):
            NativeAptTests().assert_apt_acquisition(json.loads(command)["uri"])
finally:
    NativeAptTests.tearDownClass()
