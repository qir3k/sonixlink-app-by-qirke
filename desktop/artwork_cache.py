"""
Artwork cache for SonixLink Desktop.
Fetches track artwork from /api/art, saves to disk cache,
and returns Pillow ImageTk images scaled cleanly with rounded corners for modern desktop UI.
"""

from __future__ import annotations

import hashlib
import io
import os
import tempfile
from typing import Any, Dict, Optional, Set, Tuple
from PIL import Image, ImageDraw, ImageTk


def _get_cache_dir() -> str:
    # 1. Prefer cache folder in the application root
    app_dir = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    local_cache = os.path.join(app_dir, ".artwork_cache")
    try:
        os.makedirs(local_cache, exist_ok=True)
        test_f = os.path.join(local_cache, ".write_test")
        with open(test_f, "w") as f:
            f.write("1")
        os.remove(test_f)
        return local_cache
    except Exception:
        pass

    # 2. Check LOCALAPPDATA, user home, or TEMP
    candidates = []
    localapp = os.environ.get("LOCALAPPDATA")
    if localapp:
        candidates.append(os.path.join(localapp, "SonixLink", "artwork"))
    candidates.append(os.path.join(os.path.expanduser("~"), ".sonixlink", "artwork"))
    candidates.append(os.path.join(tempfile.gettempdir(), "sonixlink", "artwork"))
    appdata = os.environ.get("APPDATA")
    if appdata:
        candidates.append(os.path.join(appdata, "SonixLink", "artwork"))

    for c in candidates:
        try:
            os.makedirs(c, exist_ok=True)
            test_f = os.path.join(c, ".write_test")
            with open(test_f, "w") as f:
                f.write("1")
            os.remove(test_f)
            return c
        except Exception:
            continue
    fallback = os.path.join(os.getcwd(), ".sonixlink_art_cache")
    os.makedirs(fallback, exist_ok=True)
    return fallback


def make_rounded_image(im: Image.Image, radius: int = 12) -> Image.Image:
    """Apply anti-aliased rounded corners to a PIL Image."""
    im = im.convert("RGBA")
    w, h = im.size
    mask = Image.new("L", (w, h), 0)
    draw = ImageDraw.Draw(mask)
    draw.rounded_rectangle([0, 0, w - 1, h - 1], radius=radius, fill=255)
    im.putalpha(mask)
    return im


class ArtworkCache:
    """Caches album covers in memory and on disk with modern rounded corners."""

    def __init__(self, client: Any = None):
        self.client = client
        self.cache_dir = _get_cache_dir()
        self._memory_cache: Dict[str, ImageTk.PhotoImage] = {}
        self._missing: Set[str] = set()

    def _cache_path(self, track_path: str) -> str:
        h = hashlib.sha256(track_path.encode("utf-8")).hexdigest()
        return os.path.join(self.cache_dir, f"{h}.jpg")

    def get_artwork(
        self,
        client: Any,
        track_path: str,
        size: Tuple[int, int] = (200, 200),
        force: bool = False,
    ) -> Optional[ImageTk.PhotoImage]:
        """Fetch artwork photo image for UI canvas."""
        if not track_path:
            return None

        w, h = size
        max_dim = max(w, h)
        if force:
            self._missing.discard(track_path)
            self._memory_cache.pop(f"{track_path}_{max_dim}", None)

        if track_path in self._missing and not force:
            return None

        cache_key = f"{track_path}_{max_dim}"
        if cache_key in self._memory_cache:
            return self._memory_cache[cache_key]

        disk_file = self._cache_path(track_path)
        img_data: Optional[bytes] = None

        if not force and os.path.exists(disk_file) and os.path.getsize(disk_file) > 0:
            try:
                with open(disk_file, "rb") as f:
                    img_data = f.read()
            except Exception:
                img_data = None

        cli = client or self.client
        if not img_data and cli:
            try:
                # Ask raw art directly from player (max_px=0 for immediate synchronous load)
                img_data = cli.get_art(track_path, max_px=0)
                if img_data and len(img_data) > 0:
                    try:
                        with open(disk_file, "wb") as f:
                            f.write(img_data)
                    except Exception:
                        pass
                else:
                    self._missing.add(track_path)
                    return None
            except Exception:
                return None

        if img_data:
            try:
                pil_img = Image.open(io.BytesIO(img_data)).convert("RGBA")
                # Smooth downsample with high-fidelity Lanczos resampling
                pil_img = pil_img.resize((w, h), Image.Resampling.LANCZOS)
                # Apply modern rounded corners
                pil_img = make_rounded_image(pil_img, radius=12)
                photo = ImageTk.PhotoImage(pil_img)
                self._memory_cache[cache_key] = photo
                return photo
            except Exception as e:
                print(f"Error decoding image: {e}")
                self._missing.add(track_path)

        return None

    def get_image(self, track_path: str, size: int = 200) -> Optional[ImageTk.PhotoImage]:
        return self.get_artwork(self.client, track_path, size=(size, size))

    def get_placeholder(self, size: int = 200) -> ImageTk.PhotoImage:
        key = f"placeholder_{size}"
        if key in self._memory_cache:
            return self._memory_cache[key]

        img = Image.new("RGBA", (size, size), (20, 22, 28, 255))
        draw = ImageDraw.Draw(img)

        # Concentric grooves
        cx, cy = size // 2, size // 2
        for r_pct in (0.90, 0.76, 0.62, 0.48):
            r = int(size * r_pct / 2)
            draw.ellipse([cx - r, cy - r, cx + r, cy + r], outline=(36, 40, 50, 255), width=1)

        # Center label disc
        cr = int(size * 0.22)
        draw.ellipse([cx - cr, cy - cr, cx + cr, cy + cr], fill=(28, 31, 40, 255), outline=(50, 55, 68, 255), width=2)
        # Center hole
        hr = int(size * 0.04)
        draw.ellipse([cx - hr, cy - hr, cx + hr, cy + hr], fill=(12, 13, 16, 255), outline=(70, 75, 90, 255), width=1)

        img = make_rounded_image(img, radius=12)
        photo = ImageTk.PhotoImage(img)
        self._memory_cache[key] = photo
        return photo

    def cached_count(self) -> int:
        try:
            if os.path.exists(self.cache_dir):
                return len([f for f in os.listdir(self.cache_dir) if f.endswith(".jpg")])
        except Exception:
            pass
        return 0

    def clear_cache(self) -> None:
        self._memory_cache.clear()
        self._missing.clear()
        try:
            if os.path.exists(self.cache_dir):
                for f in os.listdir(self.cache_dir):
                    fp = os.path.join(self.cache_dir, f)
                    if os.path.isfile(fp):
                        try:
                            os.remove(fp)
                        except Exception:
                            pass
        except Exception:
            pass

