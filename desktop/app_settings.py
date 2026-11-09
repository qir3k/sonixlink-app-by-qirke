"""
Persistent settings for SonixLink Desktop.
Stores last connected host/port, database sync stamps, and user preferences.
"""

from __future__ import annotations

import json
import os
import tempfile
from typing import Any, Dict


def _get_settings_path() -> str:
    # 1. Prefer saving right in the application / repository directory
    app_dir = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    local_settings = os.path.join(app_dir, "settings.json")
    try:
        test_f = os.path.join(app_dir, ".write_test")
        with open(test_f, "w") as f:
            f.write("1")
        os.remove(test_f)
        return local_settings
    except Exception:
        pass

    # 2. Check LOCALAPPDATA, user home, or TEMP
    candidates = []
    localapp = os.environ.get("LOCALAPPDATA")
    if localapp:
        candidates.append(os.path.join(localapp, "SonixLink"))
    candidates.append(os.path.join(os.path.expanduser("~"), ".sonixlink"))
    candidates.append(os.path.join(tempfile.gettempdir(), "sonixlink"))
    appdata = os.environ.get("APPDATA")
    if appdata:
        candidates.append(os.path.join(appdata, "SonixLink"))

    for c in candidates:
        try:
            os.makedirs(c, exist_ok=True)
            test_f = os.path.join(c, ".write_test")
            with open(test_f, "w") as f:
                f.write("1")
            os.remove(test_f)
            return os.path.join(c, "settings.json")
        except Exception:
            continue
    return os.path.join(os.getcwd(), "settings.json")


class AppSettings:
    """Manages reading and writing application preferences to JSON."""

    def __init__(self):
        self.file_path = _get_settings_path()
        self.data: Dict[str, Any] = self._load()

    def _load(self) -> Dict[str, Any]:
        if os.path.exists(self.file_path):
            try:
                with open(self.file_path, "r", encoding="utf-8") as f:
                    return json.load(f)
            except Exception:
                pass
        return {}

    def save(self) -> None:
        try:
            with open(self.file_path, "w", encoding="utf-8") as f:
                json.dump(self.data, f, indent=2)
        except Exception as e:
            print("Failed to save settings:", e)

    def get(self, key: str, default: Any = None) -> Any:
        return self.data.get(key, default)

    def set(self, key: str, value: Any) -> None:
        self.data[key] = value
        self.save()

    @property
    def last_host(self) -> str:
        # Default to 192.168.0.17 if not set
        return self.data.get("last_host", "192.168.0.17")

    @last_host.setter
    def last_host(self, host: str) -> None:
        self.set("last_host", host)

    @property
    def last_port(self) -> int:
        return int(self.data.get("last_port", 7800))

    @last_port.setter
    def last_port(self, port: int) -> None:
        self.set("last_port", port)

    @property
    def db_stamp(self) -> str:
        return self.data.get("db_stamp", "")

    @db_stamp.setter
    def db_stamp(self, stamp: str) -> None:
        self.set("db_stamp", stamp)

    @property
    def accent_preference(self) -> str:
        return self.data.get("accent_preference", "device")

    @accent_preference.setter
    def accent_preference(self, pref: str) -> None:
        self.set("accent_preference", pref)

    @property
    def device_accent(self) -> str:
        return self.data.get("device_accent", "")

    @device_accent.setter
    def device_accent(self, acc: str) -> None:
        if acc:
            self.set("device_accent", acc)

    @property
    def effective_accent(self) -> str:
        pref = self.accent_preference
        if pref and pref != "device":
            return pref.upper()
        dev = self.device_accent
        if dev:
            return dev.upper()
        return "#FF8C00"

    @property
    def fav_mark_preference(self) -> str:
        return self.data.get("fav_mark_preference", "device")

    @fav_mark_preference.setter
    def fav_mark_preference(self, pref: str) -> None:
        self.set("fav_mark_preference", pref)

    @property
    def device_fav_mark(self) -> str:
        return self.data.get("device_fav_mark", "heart")

    @device_fav_mark.setter
    def device_fav_mark(self, mark: str) -> None:
        if mark:
            self.set("device_fav_mark", mark.lower())

    @property
    def effective_fav_mark(self) -> str:
        pref = self.fav_mark_preference
        if pref in ("star", "heart"):
            return pref
        return self.device_fav_mark or "heart"

    @property
    def volume_step(self) -> int:
        return int(self.data.get("volume_step", 3))

    @volume_step.setter
    def volume_step(self, step: int) -> None:
        self.set("volume_step", max(1, min(5, int(step))))


# Global singleton instance
settings = AppSettings()
