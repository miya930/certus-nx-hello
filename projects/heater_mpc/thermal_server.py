# /// script
# requires-python = ">=3.10"
# dependencies = [
#     "numpy",
#     "scipy",
# ]
# ///
"""温度制御シミュレータの画面を配信し、画面で指定した条件でシミュレーションと最適化を実行する。

計算は時間がかかるため、POST でジョブとして受け付け、GET /api/jobs/<id> で進み具合と結果を返す。
"""

import argparse
import json
import shutil
import subprocess
import sys
import threading
import time
import traceback
import uuid
import webbrowser
from http import HTTPStatus
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

from optimize import optimize
from thermal_control import AMBIENT, PLANT_DT, simulate
from thermal_model import Geometry

HERE = Path(__file__).parent
UI_SOURCE = HERE / "ui"
UI_BUILD = HERE / "build" / "ui"
DEFAULT_PORT = 8765

# 実際の基板の役のモデルが 1 mm 格子なので、基板の辺の長さで計算時間を抑える。
MAX_BOARD_MM = 150.0
MAX_PHASES = 8
# 基板上の素子の温度の上限に合わせる。
MAX_SETPOINT = 60.0

# 結果を取りに来ないジョブを捨てるまでの時間。
JOB_TTL = 600.0  # s


class RequestError(ValueError):
    pass


def number(obj, key, low, high):
    value = obj.get(key)
    if not isinstance(value, (int, float)) or isinstance(value, bool):
        raise RequestError(f"{key} must be a number")
    if not low <= value <= high:
        raise RequestError(f"{key} must be between {low} and {high}")
    return float(value)


def integer(obj, key, low, high):
    value = number(obj, key, low, high)
    if value != int(value):
        raise RequestError(f"{key} must be an integer")
    return int(value)


def choice(obj, key, options):
    value = obj.get(key)
    if value not in options:
        raise RequestError(f"{key} must be one of {', '.join(options)}")
    return value


def parse_request(body):
    g = body.get("geometry", {})
    geometry = Geometry(
        rows=integer(g, "rows", 1, 4),
        cols=integer(g, "cols", 1, 4),
        pitch=number(g, "pitchMm", 5.0, 60.0) * 1e-3,
        margin=number(g, "marginMm", 3.0, 40.0) * 1e-3,
        heater_x=number(g, "heaterXMm", 1.0, 12.0) * 1e-3,
        heater_y=number(g, "heaterYMm", 1.0, 12.0) * 1e-3,
        sensor_offset=number(g, "sensorOffsetMm", 0.0, 15.0) * 1e-3,
        thickness=number(g, "thicknessMm", 0.4, 3.2) * 1e-3,
        copper=number(g, "copper", 0.0, 1.0),
    )
    if max(geometry.width, geometry.height) > MAX_BOARD_MM * 1e-3:
        raise RequestError(f"board must be at most {MAX_BOARD_MM:.0f} mm on each side")
    if max(geometry.heater_x, geometry.heater_y) >= geometry.pitch:
        raise RequestError("heater must be smaller than the pitch")

    power_max = number(body, "powerMax", 0.05, 2.0)

    s = body.get("scenario", {})
    duration = number(s, "duration", 300.0, 3600.0)
    ramp_rate = number(s, "rampRate", 0.0, 60.0)
    phases = s.get("phases")
    if not isinstance(phases, list) or not 1 <= len(phases) <= MAX_PHASES:
        raise RequestError(f"scenario.phases must have 1 to {MAX_PHASES} phases")
    parsed = []
    for phase in phases:
        start = number(phase, "start", 0.0, duration)
        setpoint = phase.get("setpoint")
        if not isinstance(setpoint, list) or len(setpoint) != geometry.heaters:
            raise RequestError(f"each phase needs {geometry.heaters} setpoints")
        if not all(isinstance(v, (int, float)) and AMBIENT <= v <= MAX_SETPOINT for v in setpoint):
            raise RequestError(f"setpoints must be between {AMBIENT:.0f} and {MAX_SETPOINT:.0f} degC")
        parsed.append({"start": start, "setpoint": setpoint})
    starts = sorted(p["start"] for p in parsed)
    if starts[0] != 0.0 or len(set(starts)) != len(starts):
        raise RequestError("phase starts must be distinct and the first must be 0 s")

    p = body.get("pid", {})
    pid = {"mode": choice(p, "mode", ("imc", "explicit"))}
    if pid["mode"] == "imc":
        pid.update({"lambdaRatio": number(p, "lambdaRatio", 0.1, 5.0), "optimize": bool(p.get("optimize", False))})
    else:
        pid.update({"kp": number(p, "kp", 1e-4, 10.0), "ti": number(p, "ti", 1.0, 3600.0), "td": number(p, "td", 0.0, 600.0)})

    m = body.get("mpc", {})
    horizon = integer(m, "horizon", 5, 120)
    mpc = {
        "sampleTime": integer(m, "sampleTime", int(PLANT_DT), 30),
        "horizon": horizon,
        "controlHorizon": integer(m, "controlHorizon", 1, horizon),
        "moveWeight": number(m, "moveWeight", 0.0, 1e4),
    }
    return {"geometry": geometry, "power_max": power_max, "duration": duration, "phases": parsed,
            "ramp_rate": ramp_rate, "pid": pid, "mpc": mpc}


class Job:
    """バックグラウンドのスレッドで計算を進め、進み具合と結果を保持する。"""

    def __init__(self, work):
        self.lock = threading.Lock()
        self.status = "running"
        self.stage = "starting"
        self.fraction = 0.0
        self.result = None
        self.error = None
        self.updated = time.time()
        threading.Thread(target=self._run, args=(work,), daemon=True).start()

    def _progress(self, stage, fraction):
        with self.lock:
            self.stage = stage
            self.fraction = fraction
            self.updated = time.time()

    def _run(self, work):
        try:
            result = work(self._progress)
            with self.lock:
                self.result = result
                self.status = "done"
        except Exception:
            with self.lock:
                self.error = traceback.format_exc(limit=3)
                self.status = "error"
        self.updated = time.time()

    def snapshot(self):
        with self.lock:
            payload = {"status": self.status, "stage": self.stage, "fraction": self.fraction}
            if self.status == "done":
                payload["result"] = self.result
            elif self.status == "error":
                payload["error"] = self.error
            return payload


jobs = {}
jobs_lock = threading.Lock()


def start_job(work):
    job_id = uuid.uuid4().hex
    with jobs_lock:
        expired = [k for k, j in jobs.items() if time.time() - j.updated > JOB_TTL]
        for k in expired:
            del jobs[k]
        jobs[job_id] = Job(work)
    return job_id


def build_ui():
    """画面のソースがビルドより新しいときだけ、npm でビルドし直す。"""
    sources = [p for p in UI_SOURCE.rglob("*") if p.is_file() and "node_modules" not in p.parts]
    index = UI_BUILD / "index.html"
    if index.exists() and index.stat().st_mtime >= max(p.stat().st_mtime for p in sources):
        return
    npm = shutil.which("npm")
    if npm is None:
        sys.exit("npm is required to build the UI")
    if not (UI_SOURCE / "node_modules").exists():
        subprocess.run([npm, "ci"], cwd=UI_SOURCE, check=True)
    subprocess.run([npm, "run", "build"], cwd=UI_SOURCE, check=True)


class Handler(SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=str(UI_BUILD), **kwargs)

    def do_GET(self):
        if not self.path.startswith("/api/jobs/"):
            super().do_GET()
            return
        with jobs_lock:
            job = jobs.get(self.path[len("/api/jobs/"):])
        if job is None:
            self.send_json(HTTPStatus.NOT_FOUND, {"error": "unknown job"})
            return
        snapshot = job.snapshot()
        if snapshot["status"] != "running":
            with jobs_lock:
                jobs.pop(self.path[len("/api/jobs/"):], None)
        self.send_json(HTTPStatus.OK, snapshot)

    def do_POST(self):
        if self.path not in ("/api/simulate", "/api/optimize"):
            self.send_error(HTTPStatus.NOT_FOUND)
            return
        try:
            body = json.loads(self.rfile.read(int(self.headers.get("Content-Length", 0))))
            request = parse_request(body)
            if self.path == "/api/optimize":
                target = choice(body, "target", ("pid", "mpc", "both"))
        except (RequestError, ValueError, AttributeError) as error:
            self.send_json(HTTPStatus.BAD_REQUEST, {"error": str(error)})
            return
        if self.path == "/api/optimize":
            work = lambda progress: optimize(**request, target=target, progress=progress)
        else:
            work = lambda progress: simulate(**request, progress=progress)
        self.send_json(HTTPStatus.ACCEPTED, {"job": start_job(work)})

    def send_json(self, status, payload):
        body = json.dumps(payload).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, format, *args):
        # 進み具合の問い合わせでログが埋まらないよう、ジョブの GET は記録しない。
        if not str(args[0]).startswith("GET /api/jobs/"):
            super().log_message(format, *args)


def open_in_browser(url):
    # WSL では Linux 側にブラウザがないため、Windows のブラウザで開く。
    if shutil.which("explorer.exe"):
        subprocess.run(["explorer.exe", url])
    else:
        webbrowser.open(url)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--port", type=int, default=DEFAULT_PORT)
    parser.add_argument("--no-open", action="store_true", help="do not open the simulator in a browser")
    args = parser.parse_args()

    build_ui()
    server = ThreadingHTTPServer(("127.0.0.1", args.port), Handler)
    url = f"http://localhost:{args.port}/"
    print(f"serving: {url}")
    if not args.no_open:
        open_in_browser(url)
    server.serve_forever()


if __name__ == "__main__":
    main()
