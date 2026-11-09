"""
SonixLink HTTP client for Windows Desktop.
Connects to SonixPlayer via HTTP API on port 7800.
"""

from __future__ import annotations

from dataclasses import dataclass
import json
import os
import time
import urllib.error
import urllib.parse
import urllib.request
from typing import Any, Dict, List, Optional

DEFAULT_PORT = 7800


@dataclass
class PlayerInfo:
    name: str = "SonixPlayer"
    model: str = "TempoTec V1"
    firmware: str = ""
    serial: str = ""
    tracks: int = 0
    scanning: bool = False
    db_available: bool = False
    db_size: int = 0
    db_mtime: int = 0
    db_hash: str = ""
    covers_hash: str = ""
    accent: str = "#3584E4"

    @property
    def stamp(self) -> str:
        """Unique fingerprint for database versioning."""
        return f"{self.serial}_{self.db_hash}_{self.db_mtime}_{self.tracks}"


@dataclass
class PlayerState:
    state: int = 0  # 0: stopped, 1: playing, 2: paused
    mode: str = "normal"  # normal, repeat_all, repeat_one, shuffle, shuffle_repeat
    volume: int = 0
    volume_step: int = 3
    position: int = 0
    duration: int = 0
    title: str = ""
    artist: str = ""
    album: str = ""
    album_artist: str = ""
    path: str = ""
    sample_rate: int = 0
    bitrate: int = 0
    bits: int = 0
    lossless: bool = False
    battery: int = -1
    charging: bool = False
    favourite: bool = False
    fav_star: Optional[bool] = None
    fav_mark: str = ""
    accent: str = "#3584E4"
    queue_position: int = -1
    queue_count: int = 0
    queue_revision: int = 0
    display_position: int = -1
    display_count: int = -1
    podcast: bool = False
    podcast_speed: int = 1000
    mseb: bool = False
    eq: bool = False
    peq: bool = False
    peq_profile: int = 1
    scanning: bool = False
    muted: bool = False

    @property
    def is_playing(self) -> bool:
        return self.state == 1

    @property
    def has_track(self) -> bool:
        return bool(self.title or self.path)

    @property
    def is_favorite(self) -> bool:
        return self.favourite

    @property
    def accent_color(self) -> str:
        return self.accent

    @property
    def queue_index(self) -> int:
        return self.queue_position

    @property
    def queue_total(self) -> int:
        return self.queue_count

    @property
    def playback_mode(self) -> str:
        return self.mode

    @property
    def audio_format_badge(self) -> str:
        """Formatted audio quality badge, e.g. 'FLAC 96kHz / 24bit' or '320 kbps'."""
        ext = os.path.splitext(self.path)[1].upper().lstrip(".") if self.path else ""
        parts = []
        if ext:
            parts.append(ext)
        elif self.lossless:
            parts.append("Lossless")

        if self.sample_rate > 0:
            if self.sample_rate >= 1000:
                parts.append(f"{self.sample_rate / 1000:.1f}".rstrip(".0") + "kHz")
            else:
                parts.append(f"{self.sample_rate}Hz")

        if self.bits > 0:
            parts.append(f"{self.bits}bit")
        elif self.bitrate > 0:
            parts.append(f"{self.bitrate}kbps")

        return " • ".join(parts) if parts else ""


class SonixLinkClient:
    def __init__(self, host: str = "127.0.0.1", port: int = DEFAULT_PORT):
        self.host = host
        self.port = port
        self.base_url = f"http://{host}:{port}"
        self.last_state: Optional[PlayerState] = None
        self._last_success_time: float = 0.0

    def set_target(self, host: str, port: int = DEFAULT_PORT) -> None:
        self.host = host
        self.port = port
        self.base_url = f"http://{host}:{port}"

    @property
    def is_connected(self) -> bool:
        return (time.time() - self._last_success_time) < 4.0

    def _url(self, path: str) -> str:
        return f"{self.base_url}{path}"

    def _get_json(self, path: str, timeout: float = 2.5) -> Optional[dict]:
        try:
            url = self._url(path)
            req = urllib.request.Request(url, headers={"User-Agent": "SonixLink-Desktop/1.0"})
            with urllib.request.urlopen(req, timeout=timeout) as response:
                data = response.read().decode("utf-8")
                self._last_success_time = time.time()
                return json.loads(data)
        except Exception:
            return None

    def _post(self, path: str, timeout: float = 2.5, data: Optional[bytes] = None) -> Optional[bytes]:
        try:
            url = self._url(path)
            req = urllib.request.Request(
                url,
                data=data or b"",
                headers={"User-Agent": "SonixLink-Desktop/1.0"},
                method="POST",
            )
            with urllib.request.urlopen(req, timeout=timeout) as response:
                self._last_success_time = time.time()
                return response.read()
        except Exception:
            return None

    def get_info(self) -> Optional[PlayerInfo]:
        raw = self._get_json("/api/info")
        if not raw:
            return None
        return PlayerInfo(
            name=raw.get("name", self.host),
            model=raw.get("model", ""),
            firmware=raw.get("firmware", ""),
            serial=raw.get("serial", ""),
            tracks=raw.get("tracks", 0),
            scanning=raw.get("scanning", False),
            db_available=raw.get("db_available", False),
            db_size=raw.get("db_size", 0),
            db_mtime=raw.get("db_mtime", 0),
            db_hash=raw.get("db_hash", ""),
            covers_hash=raw.get("covers_hash", ""),
            accent=raw.get("accent", "#3584E4"),
        )

    def get_state(self) -> Optional[PlayerState]:
        raw = self._get_json("/api/state")
        if not raw:
            return self.last_state

        state = PlayerState(
            state=raw.get("state", 0),
            mode=raw.get("mode", "normal"),
            volume=raw.get("volume", 0),
            volume_step=raw.get("volume_step", 3),
            position=raw.get("position", 0),
            duration=raw.get("duration", 0),
            title=raw.get("title", ""),
            artist=raw.get("artist", ""),
            album=raw.get("album", ""),
            album_artist=raw.get("album_artist", ""),
            path=raw.get("path", ""),
            sample_rate=raw.get("sample_rate", 0),
            bitrate=raw.get("bitrate", 0),
            bits=raw.get("bits", 0),
            lossless=raw.get("lossless", False),
            battery=raw.get("battery", -1),
            charging=raw.get("charging", False),
            favourite=raw.get("favourite", False),
            fav_star=raw.get("fav_star"),
            fav_mark=raw.get("fav_mark", ""),
            accent=raw.get("accent", "#3584E4"),
            queue_position=raw.get("queue_position", -1),
            queue_count=raw.get("queue_count", 0),
            queue_revision=raw.get("queue_revision", 0),
            display_position=raw.get("display_position", -1),
            display_count=raw.get("display_count", -1),
            podcast=raw.get("podcast", False),
            podcast_speed=raw.get("podcast_speed", 1000),
            mseb=raw.get("mseb", False),
            eq=raw.get("eq", False),
            peq=raw.get("peq", False),
            peq_profile=raw.get("peq_profile", 1),
            scanning=raw.get("scanning", False),
            muted=raw.get("volume", 0) == 0,
        )
        self.last_state = state
        return state

    def command(self, do: str, **params) -> None:
        q = [("do", do)]
        for k, v in params.items():
            if v is not None:
                q.append((k, str(v)))
        query_str = urllib.parse.urlencode(q)
        self._post(f"/api/command?{query_str}")

    def play(self) -> None:
        self.command("play")

    def pause(self) -> None:
        self.command("pause")

    def toggle(self) -> None:
        self.command("toggle")

    def toggle_play_pause(self) -> None:
        self.toggle()

    def next(self) -> None:
        self.command("next")

    def next_track(self) -> None:
        self.next()

    def prev(self) -> None:
        self.command("prev")

    def prev_track(self) -> None:
        self.prev()

    def stop(self) -> None:
        self.command("stop")

    def seek(self, seconds: int) -> None:
        self.command("seek", value=seconds)

    def set_volume(self, percent: int) -> None:
        self.command("volume", value=max(0, min(100, percent)))

    def toggle_mute(self) -> None:
        if self.last_state and self.last_state.volume > 0:
            self._prev_vol = self.last_state.volume
            self.set_volume(0)
        else:
            vol = getattr(self, "_prev_vol", 35)
            self.set_volume(vol)

    def set_volume_step(self, step: int) -> None:
        self.command("volume_step", value=max(1, min(5, step)))

    def set_mode(self, mode: str) -> None:
        self.command("mode", value=mode)

    def cycle_playback_mode(self) -> None:
        modes = ["normal", "repeat", "single", "shuffle"]
        cur = self.last_state.mode if self.last_state else "normal"
        try:
            idx = modes.index(cur)
            next_m = modes[(idx + 1) % len(modes)]
        except ValueError:
            next_m = "normal"
        self.set_mode(next_m)

    def set_favourite(self, path: str, starred: bool) -> None:
        self.command("favourite", path=path, value=1 if starred else 0)

    def toggle_favourite(self, path: str) -> None:
        self.command("favourite", path=path)

    def toggle_favorite(self, path: str) -> None:
        self.toggle_favourite(path)

    def set_speed(self, permille: int) -> None:
        self.command("speed", value=max(250, min(4000, permille)))

    # --- DSP Controls ---
    def set_mseb(self, enabled: bool) -> None:
        self.command("mseb", value=1 if enabled else 0)

    def toggle_mseb(self) -> None:
        self.command("mseb")

    def set_eq(self, enabled: bool) -> None:
        self.command("eq", value=1 if enabled else 0)

    def toggle_eq(self) -> None:
        self.command("eq")

    def set_peq(self, enabled: bool) -> None:
        self.command("peq", value=1 if enabled else 0)

    def toggle_peq(self) -> None:
        self.command("peq")

    def set_peq_profile(self, profile: int) -> None:
        self.command("peq_profile", value=max(1, min(2, profile)))

    def next_peq_profile(self) -> None:
        self.command("peq_profile", value="next")

    # --- Library & Queue Playback ---
    def play_path(self, path: str, list_type: str = "all", value: str = "") -> None:
        self.command("play_path", path=path, list=list_type, value=value)

    def enqueue(self, path: str, play_next: bool = False) -> None:
        action = "play_next" if play_next else "enqueue"
        self.command(action, path=path)

    def play_all(self, list_type: str, value: str = "", how: str = "sequence") -> None:
        self.command("play_all", list=list_type, value=value, how=how)

    def play_queue_index(self, index: int) -> None:
        self.command("queue_index", value=index)

    def start_scan(self) -> None:
        self.command("scan")

    def bye(self) -> None:
        try:
            req = urllib.request.Request(self._url("/api/bye"))
            urllib.request.urlopen(req, timeout=1.5)
        except Exception:
            pass

    def get_queue(self, from_index: int = 0) -> List[Dict[str, Any]]:
        raw = self._get_json(f"/api/queue?from={from_index}")
        if not raw:
            return []
        items = raw.get("items", [])
        return items if isinstance(items, list) else []

    def browse_directory(self, path: str = "") -> Dict[str, Any]:
        return self.get_browse(path)

    def get_browse(self, path: str = "") -> Dict[str, Any]:
        q = urllib.parse.urlencode({"path": path}) if path else ""
        res = self._get_json(f"/api/browse?{q}" if q else "/api/browse")
        return res or {"items": []}

    def get_stream_services(self) -> Dict[str, Any]:
        res = self._get_json("/api/stream/services")
        return res or {}

    def get_stream_list(self, service: str, kind: str = "menu", id_val: str = "", q: str = "", offset: int = 0) -> Dict[str, Any]:
        params = {"service": service, "kind": kind, "id": id_val, "q": q, "offset": offset}
        query = urllib.parse.urlencode(params)
        res = self._get_json(f"/api/stream/list?{query}")
        return res or {}

    def stream_play(self, service: str, index: int) -> None:
        params = {"service": service, "index": str(index)}
        query = urllib.parse.urlencode(params)
        self._get_json(f"/api/stream/play?{query}")

    def get_art(self, path: str, max_px: int = 0) -> Optional[bytes]:
        """
        Fetch album artwork for a file.
        If max_px is 0, firmware returns raw embedded art directly (fastest).
        Handles HTTP 202 Accepted ('preparing') by retrying every 200ms up to 10 times.
        """
        if not path:
            return None

        params = {"path": path}
        if max_px > 0:
            params["max"] = str(max_px)
        q = urllib.parse.urlencode(params)
        url = self._url(f"/api/art?{q}")

        retries = 10
        for _ in range(retries):
            try:
                req = urllib.request.Request(url, headers={"User-Agent": "SonixLink-Desktop/1.0"})
                with urllib.request.urlopen(req, timeout=3.5) as resp:
                    if resp.status == 200:
                        data = resp.read()
                        if len(data) > 0:
                            return data
                    elif resp.status == 202:
                        # Firmware is preparing the cover asynchronously
                        time.sleep(0.2)
                        continue
                    elif resp.status in (404, 204):
                        return None
            except urllib.error.HTTPError as e:
                if e.code == 202:
                    time.sleep(0.2)
                    continue
                if e.code in (404, 204):
                    return None
                break
            except Exception:
                break
        return None

    def download_database_to_file(self, dest_path: str, progress_cb=None) -> bool:
        """Download SQLite index to .part file and atomically replace dest_path."""
        part_path = dest_path + ".part"
        try:
            url = self._url("/api/db")
            req = urllib.request.Request(url, headers={"User-Agent": "SonixLink-Desktop/1.0"})
            with urllib.request.urlopen(req, timeout=40.0) as resp:
                if resp.status != 200:
                    return False
                total_size = int(resp.headers.get("Content-Length", 0))
                downloaded = 0
                chunk_size = 65536
                with open(part_path, "wb") as f:
                    while True:
                        chunk = resp.read(chunk_size)
                        if not chunk:
                            break
                        f.write(chunk)
                        downloaded += len(chunk)
                        if progress_cb and total_size > 0:
                            progress_cb(downloaded, total_size)

            if os.path.exists(dest_path):
                try:
                    os.remove(dest_path)
                except Exception:
                    pass
            os.replace(part_path, dest_path)
            return True
        except Exception as e:
            print("Error downloading database:", e)
            if os.path.exists(part_path):
                try:
                    os.remove(part_path)
                except Exception:
                    pass
            return False

    def download_database(self) -> bytes:
        try:
            url = self._url("/api/db")
            req = urllib.request.Request(url, headers={"User-Agent": "SonixLink-Desktop/1.0"})
            with urllib.request.urlopen(req, timeout=30.0) as resp:
                if resp.status == 200:
                    return resp.read()
        except Exception:
            pass
        return b""

    def download_db(self, dest_path: str, progress_cb=None) -> bool:
        return self.download_database_to_file(dest_path, progress_cb=progress_cb)
