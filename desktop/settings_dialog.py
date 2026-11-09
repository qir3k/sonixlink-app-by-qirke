"""
Settings dialog for SonixLink Desktop.
Matches the Android app's preferences:
- Accent Color chooser (From player + 14 preset colors)
- Favourite Icon style (Auto / Heart / Star)
- Volume step (1% - 5%)
- Library & Database Sync
- Artwork Cache Cleaner
- Audio & DSP settings shortcut
- Device connection info and network scan
"""

from __future__ import annotations

import os
import tkinter as tk
from tkinter import messagebox, ttk
from typing import Any, Callable, Optional

from app_settings import settings
from ui_theme import THEME, ThemeManager

# Android app accent color palette
ACCENT_PRESETS = [
    ("device", "From player (device default)", None),
    ("#3584e4", "Blue", "#3584E4"),
    ("#e01b24", "Red", "#E01B24"),
    ("#ff7800", "Orange", "#FF7800"),
    ("#e5a50a", "Yellow", "#E5A50A"),
    ("#99c500", "Lime", "#99C500"),
    ("#2ec27e", "Green", "#2EC27E"),
    ("#00b0d6", "Cyan", "#00B0D6"),
    ("#665be0", "Indigo", "#665BE0"),
    ("#9141ac", "Purple", "#9141AC"),
    ("#d56199", "Pink", "#D56199"),
    ("#a86530", "Brown", "#A86530"),
    ("#8b8e94", "Slate", "#8B8E94"),
    ("#ffad2e", "Amber", "#FFAD2E"),
    ("#ffffff", "White (OLED)", "#FFFFFF"),
]


class SettingsDialog(tk.Toplevel):
    """Modern dark-themed Settings window matching Android app features."""

    def __init__(
        self,
        parent: tk.Tk,
        client: Any,
        theme_mgr: ThemeManager,
        artwork_cache: Any,
        library_db: Any,
        on_accent_changed: Optional[Callable[[], None]] = None,
        on_fav_mark_changed: Optional[Callable[[], None]] = None,
        on_resync_db: Optional[Callable[[], None]] = None,
        on_open_dsp: Optional[Callable[[], None]] = None,
        on_open_connect: Optional[Callable[[], None]] = None,
    ):
        super().__init__(parent)
        self.client = client
        self.theme_mgr = theme_mgr
        self.artwork_cache = artwork_cache
        self.library_db = library_db
        self.on_accent_changed = on_accent_changed
        self.on_fav_mark_changed = on_fav_mark_changed
        self.on_resync_db = on_resync_db
        self.on_open_dsp = on_open_dsp
        self.on_open_connect = on_open_connect

        self.title("Settings - SonixLink")
        self.geometry("680x760")
        self.minsize(580, 500)
        self.configure(bg=THEME["bg_base"])

        # Center on parent window
        self.transient(parent)
        self.grab_set()
        self._center_on_parent(parent)

        # Set titlebar icon if available
        icon_path = os.path.join(os.path.dirname(__file__), "icon.ico")
        if os.path.exists(icon_path):
            try:
                self.iconbitmap(icon_path)
            except Exception:
                pass

        self.accent_widgets: list[tuple[str, tk.Widget, Any]] = []
        self.fav_widgets: list[tuple[str, tk.Widget]] = []
        self.vol_widgets: list[tuple[int, tk.Widget]] = []

        self._build_ui()

    def _center_on_parent(self, parent: tk.Tk) -> None:
        try:
            self.update_idletasks()
            pw = parent.winfo_width()
            ph = parent.winfo_height()
            px = parent.winfo_rootx()
            py = parent.winfo_rooty()
            w, h = 680, 760
            x = px + max(0, (pw - w) // 2)
            y = py + max(0, (ph - h) // 2)
            self.geometry(f"{w}x{h}+{x}+{y}")
        except Exception:
            pass

    def _build_ui(self) -> None:
        # Top Header Bar
        top_bar = tk.Frame(self, bg=THEME["bg_panel"], height=52)
        top_bar.pack(fill=tk.X, side=tk.TOP)
        top_bar.pack_propagate(False)

        title_lbl = tk.Label(
            top_bar,
            text="⚙️   Preferences & Settings",
            bg=THEME["bg_panel"],
            fg=THEME["text_primary"],
            font=("Segoe UI", 12, "bold"),
        )
        title_lbl.pack(side=tk.LEFT, padx=18, pady=12)

        btn_close = tk.Button(
            top_bar,
            text="✕",
            command=self.destroy,
            bg=THEME["bg_panel"],
            fg=THEME["text_secondary"],
            activebackground=THEME["bg_card"],
            activeforeground="#FFFFFF",
            relief=tk.FLAT,
            font=("Segoe UI", 11),
            cursor="hand2",
            padx=10,
        )
        btn_close.pack(side=tk.RIGHT, padx=12, pady=10)

        # Bottom Bar
        bottom_bar = tk.Frame(self, bg=THEME["bg_panel"], height=50)
        bottom_bar.pack(fill=tk.X, side=tk.BOTTOM)
        bottom_bar.pack_propagate(False)

        btn_done = tk.Button(
            bottom_bar,
            text="Done",
            command=self.destroy,
            bg=self.theme_mgr.accent_color,
            fg="#FFFFFF",
            activebackground="#FFFFFF",
            activeforeground="#000000",
            font=("Segoe UI", 9, "bold"),
            relief=tk.FLAT,
            padx=20,
            pady=6,
            cursor="hand2",
        )
        btn_done.pack(side=tk.RIGHT, padx=16, pady=8)
        self.btn_done = btn_done

        # Scrollable Body Container
        canvas_frame = tk.Frame(self, bg=THEME["bg_base"])
        canvas_frame.pack(fill=tk.BOTH, expand=True)

        self.canvas = tk.Canvas(
            canvas_frame,
            bg=THEME["bg_base"],
            highlightthickness=0,
            borderwidth=0,
        )
        scrollbar = ttk.Scrollbar(canvas_frame, orient="vertical", command=self.canvas.yview)
        self.scroll_content = tk.Frame(self.canvas, bg=THEME["bg_base"])

        self.scroll_content.bind(
            "<Configure>",
            lambda e: self.canvas.configure(scrollregion=self.canvas.bbox("all")),
        )
        self.canvas_win = self.canvas.create_window((0, 0), window=self.scroll_content, anchor="nw")
        self.canvas.configure(yscrollcommand=scrollbar.set)

        self.canvas.pack(side=tk.LEFT, fill=tk.BOTH, expand=True)
        scrollbar.pack(side=tk.RIGHT, fill=tk.Y)

        # Resize scroll window width when canvas width changes
        self.canvas.bind(
            "<Configure>",
            lambda e: self.canvas.itemconfig(self.canvas_win, width=e.width),
        )

        # Mousewheel scrolling
        self.bind_all("<MouseWheel>", self._on_mousewheel)

        # Populate Sections
        content = self.scroll_content
        self._build_accent_section(content)
        self._build_fav_mark_section(content)
        self._build_volume_step_section(content)
        self._build_library_cache_section(content)
        self._build_connection_section(content)
        self._build_about_section(content)

    def _on_mousewheel(self, event: tk.Event) -> None:
        try:
            self.canvas.yview_scroll(int(-1 * (event.delta / 120)), "units")
        except Exception:
            pass

    def _make_card(self, parent: tk.Widget, title: str, subtitle: str = "") -> tk.Frame:
        card = tk.Frame(
            parent,
            bg=THEME["bg_card"],
            highlightbackground=THEME["border"],
            highlightthickness=1,
            padx=16,
            pady=14,
        )
        card.pack(fill=tk.X, padx=16, pady=(12, 0))

        hdr = tk.Frame(card, bg=THEME["bg_card"])
        hdr.pack(fill=tk.X, pady=(0, 10))

        lbl_title = tk.Label(
            hdr,
            text=title,
            bg=THEME["bg_card"],
            fg=THEME["text_primary"],
            font=("Segoe UI", 10, "bold"),
            anchor="w",
        )
        lbl_title.pack(fill=tk.X)

        if subtitle:
            lbl_sub = tk.Label(
                hdr,
                text=subtitle,
                bg=THEME["bg_card"],
                fg=THEME["text_muted"],
                font=("Segoe UI", 8),
                anchor="w",
            )
            lbl_sub.pack(fill=tk.X, pady=(2, 0))

        return card

    # -------------------------------------------------------------------------
    # 1. Accent Color Section
    # -------------------------------------------------------------------------
    def _build_accent_section(self, parent: tk.Widget) -> None:
        card = self._make_card(
            parent,
            "🎨   Accent Color",
            "Choose your application accent color or dynamically follow the player theme.",
        )

        current_pref = settings.accent_preference

        # "From player" option (full width row)
        dev_accent = settings.device_accent or THEME["accent_default"]
        player_frame = tk.Frame(
            card,
            bg=THEME["bg_panel"] if current_pref == "device" else THEME["bg_card"],
            highlightbackground=self.theme_mgr.accent_color if current_pref == "device" else THEME["border"],
            highlightthickness=1,
            padx=10,
            pady=8,
            cursor="hand2",
        )
        player_frame.pack(fill=tk.X, pady=(0, 8))

        swatch = tk.Canvas(player_frame, width=20, height=20, bg=player_frame["bg"], highlightthickness=0)
        swatch.pack(side=tk.LEFT, padx=(0, 10))
        swatch.create_oval(2, 2, 18, 18, fill=dev_accent, outline="#FFFFFF", width=1)

        txt_box = tk.Frame(player_frame, bg=player_frame["bg"])
        txt_box.pack(side=tk.LEFT, fill=tk.X, expand=True)

        lbl_name = tk.Label(
            txt_box,
            text="From player (device default)",
            bg=player_frame["bg"],
            fg=THEME["text_primary"],
            font=("Segoe UI", 9, "bold" if current_pref == "device" else "normal"),
            anchor="w",
        )
        lbl_name.pack(fill=tk.X)

        lbl_desc = tk.Label(
            txt_box,
            text=f"Dynamically follows player theme (currently: {dev_accent.upper()})",
            bg=player_frame["bg"],
            fg=THEME["text_secondary"],
            font=("Segoe UI", 8),
            anchor="w",
        )
        lbl_desc.pack(fill=tk.X)

        lbl_check = tk.Label(
            player_frame,
            text="✓" if current_pref == "device" else "",
            bg=player_frame["bg"],
            fg=self.theme_mgr.accent_color,
            font=("Segoe UI", 12, "bold"),
        )
        lbl_check.pack(side=tk.RIGHT, padx=6)

        def pick_device():
            self._select_accent("device")

        for w in (player_frame, txt_box, lbl_name, lbl_desc, swatch, lbl_check):
            w.bind("<Button-1>", lambda e: pick_device())

        self.accent_widgets.append(("device", player_frame, lbl_check))

        # 14 Fixed Color Presets Grid (3 columns)
        grid_frame = tk.Frame(card, bg=THEME["bg_card"])
        grid_frame.pack(fill=tk.X)

        for col in range(3):
            grid_frame.columnconfigure(col, weight=1, uniform="accent_col")

        preset_items = [p for p in ACCENT_PRESETS if p[0] != "device"]
        for idx, (cid, name, hex_val) in enumerate(preset_items):
            row = idx // 3
            col = idx % 3
            is_active = (current_pref == cid)

            tile = tk.Frame(
                grid_frame,
                bg=THEME["bg_panel"] if is_active else THEME["bg_card"],
                highlightbackground=hex_val if is_active else THEME["border"],
                highlightthickness=1,
                padx=8,
                pady=6,
                cursor="hand2",
            )
            tile.grid(row=row, column=col, padx=4, pady=4, sticky="nsew")

            t_swatch = tk.Canvas(tile, width=16, height=16, bg=tile["bg"], highlightthickness=0)
            t_swatch.pack(side=tk.LEFT, padx=(0, 6))
            t_swatch.create_oval(1, 1, 15, 15, fill=hex_val, outline="#FFFFFF", width=1)

            t_lbl = tk.Label(
                tile,
                text=name,
                bg=tile["bg"],
                fg=THEME["text_primary"],
                font=("Segoe UI", 8, "bold" if is_active else "normal"),
                anchor="w",
            )
            t_lbl.pack(side=tk.LEFT, fill=tk.X, expand=True)

            t_chk = tk.Label(
                tile,
                text="✓" if is_active else "",
                bg=tile["bg"],
                fg=hex_val,
                font=("Segoe UI", 9, "bold"),
            )
            t_chk.pack(side=tk.RIGHT, padx=2)

            def make_handler(color_id=cid):
                return lambda e: self._select_accent(color_id)

            for w in (tile, t_swatch, t_lbl, t_chk):
                w.bind("<Button-1>", make_handler())

            self.accent_widgets.append((cid, tile, t_chk))

    def _select_accent(self, pref_id: str) -> None:
        settings.accent_preference = pref_id
        effective = settings.effective_accent
        self.theme_mgr.set_accent(effective)
        self.btn_done.config(bg=effective)

        # Refresh accent card highlights
        for cid, widget, chk in self.accent_widgets:
            is_active = (cid == pref_id)
            color = effective if is_active else THEME["border"]
            widget.config(
                bg=THEME["bg_panel"] if is_active else THEME["bg_card"],
                highlightbackground=color,
            )
            chk.config(
                text="✓" if is_active else "",
                bg=widget["bg"],
                fg=effective,
            )
            # Rebind children background
            for c in widget.winfo_children():
                try:
                    c.config(bg=widget["bg"])
                except Exception:
                    pass

        if self.on_accent_changed:
            self.on_accent_changed()

    # -------------------------------------------------------------------------
    # 2. Favourite Icon Style Section
    # -------------------------------------------------------------------------
    def _build_fav_mark_section(self, parent: tk.Widget) -> None:
        card = self._make_card(
            parent,
            "⭐ / ♥   Favourite Icon Style",
            "Choose the symbol used for marking favourites across the app.",
        )

        current_pref = settings.fav_mark_preference
        dev_mark = settings.device_fav_mark.capitalize()

        options = [
            ("device", "Auto (from player)", f"Follows player choice (currently: {dev_mark})", "🔄"),
            ("heart", "Heart", "Use heart symbol (♥ / ♡) for favourite tracks and playlists", "♥"),
            ("star", "Star", "Use star symbol (★ / ☆) for favourite tracks and playlists", "★"),
        ]

        for opt_id, title, desc, icon in options:
            is_active = (current_pref == opt_id)

            row = tk.Frame(
                card,
                bg=THEME["bg_panel"] if is_active else THEME["bg_card"],
                highlightbackground=self.theme_mgr.accent_color if is_active else THEME["border"],
                highlightthickness=1,
                padx=12,
                pady=8,
                cursor="hand2",
            )
            row.pack(fill=tk.X, pady=3)

            lbl_icon = tk.Label(
                row,
                text=icon,
                bg=row["bg"],
                fg=self.theme_mgr.accent_color if is_active else THEME["text_secondary"],
                font=("Segoe UI", 12, "bold"),
                width=3,
            )
            lbl_icon.pack(side=tk.LEFT, padx=(0, 8))

            txt_box = tk.Frame(row, bg=row["bg"])
            txt_box.pack(side=tk.LEFT, fill=tk.X, expand=True)

            lbl_title = tk.Label(
                txt_box,
                text=title,
                bg=row["bg"],
                fg=THEME["text_primary"],
                font=("Segoe UI", 9, "bold" if is_active else "normal"),
                anchor="w",
            )
            lbl_title.pack(fill=tk.X)

            lbl_desc = tk.Label(
                txt_box,
                text=desc,
                bg=row["bg"],
                fg=THEME["text_muted"],
                font=("Segoe UI", 8),
                anchor="w",
            )
            lbl_desc.pack(fill=tk.X)

            lbl_check = tk.Label(
                row,
                text="✓" if is_active else "",
                bg=row["bg"],
                fg=self.theme_mgr.accent_color,
                font=("Segoe UI", 11, "bold"),
            )
            lbl_check.pack(side=tk.RIGHT, padx=6)

            def make_handler(chosen_id=opt_id):
                return lambda e: self._select_fav_mark(chosen_id)

            for w in (row, lbl_icon, txt_box, lbl_title, lbl_desc, lbl_check):
                w.bind("<Button-1>", make_handler())

            self.fav_widgets.append((opt_id, row))

    def _select_fav_mark(self, mark_id: str) -> None:
        settings.fav_mark_preference = mark_id

        # Update visuals
        for mid, row in self.fav_widgets:
            is_active = (mid == mark_id)
            color = self.theme_mgr.accent_color if is_active else THEME["border"]
            row.config(
                bg=THEME["bg_panel"] if is_active else THEME["bg_card"],
                highlightbackground=color,
            )
            for c in row.winfo_children():
                try:
                    c.config(bg=row["bg"])
                except Exception:
                    pass
            # Update checkmark
            chk = row.winfo_children()[-1]
            if isinstance(chk, tk.Label):
                chk.config(text="✓" if is_active else "", fg=self.theme_mgr.accent_color)

        if self.on_fav_mark_changed:
            self.on_fav_mark_changed()

    # -------------------------------------------------------------------------
    # 3. Volume Step Section
    # -------------------------------------------------------------------------
    def _build_volume_step_section(self, parent: tk.Widget) -> None:
        card = self._make_card(
            parent,
            "🔊   Volume Step & Audio",
            "Hardware volume increment per press and quick DSP settings.",
        )

        current_step = settings.volume_step

        row_steps = tk.Frame(card, bg=THEME["bg_card"])
        row_steps.pack(fill=tk.X, pady=(0, 10))

        lbl_step = tk.Label(
            row_steps,
            text="Volume step:",
            bg=THEME["bg_card"],
            fg=THEME["text_secondary"],
            font=("Segoe UI", 9),
        )
        lbl_step.pack(side=tk.LEFT, padx=(0, 10))

        for step in (1, 2, 3, 4, 5):
            is_active = (step == current_step)
            btn = tk.Button(
                row_steps,
                text=f"{step}%",
                command=lambda s=step: self._select_volume_step(s),
                bg=self.theme_mgr.accent_color if is_active else THEME["bg_panel"],
                fg="#FFFFFF" if is_active else THEME["text_primary"],
                activebackground=self.theme_mgr.accent_color,
                activeforeground="#FFFFFF",
                font=("Segoe UI", 8, "bold" if is_active else "normal"),
                relief=tk.FLAT,
                padx=10,
                pady=4,
                cursor="hand2",
            )
            btn.pack(side=tk.LEFT, padx=3)
            self.vol_widgets.append((step, btn))

        # Audio & DSP Shortcut button
        btn_dsp = tk.Button(
            card,
            text="🎛️   Open Hardware Equalizer & DSP Settings (MSEB / EQ / PEQ)  ›",
            command=self._handle_open_dsp,
            bg=THEME["bg_panel"],
            fg=self.theme_mgr.accent_color,
            activebackground=THEME["border"],
            activeforeground="#FFFFFF",
            font=("Segoe UI", 9, "bold"),
            relief=tk.FLAT,
            padx=12,
            pady=8,
            cursor="hand2",
            anchor="w",
        )
        btn_dsp.pack(fill=tk.X, pady=(4, 0))

    def _select_volume_step(self, step: int) -> None:
        settings.volume_step = step
        try:
            self.client.set_volume_step(step)
        except Exception:
            pass

        for s, btn in self.vol_widgets:
            is_active = (s == step)
            btn.config(
                bg=self.theme_mgr.accent_color if is_active else THEME["bg_panel"],
                fg="#FFFFFF" if is_active else THEME["text_primary"],
                font=("Segoe UI", 8, "bold" if is_active else "normal"),
            )

    def _handle_open_dsp(self) -> None:
        if self.on_open_dsp:
            self.on_open_dsp()

    # -------------------------------------------------------------------------
    # 4. Library & Cache Section
    # -------------------------------------------------------------------------
    def _build_library_cache_section(self, parent: tk.Widget) -> None:
        card = self._make_card(
            parent,
            "🗄️   Library Database & Cache",
            "Manage local track metadata database and album cover image cache.",
        )

        db_path = getattr(self.library_db, "db_path", "Local SQLite Database")
        tracks_count = self.library_db.get_track_count()
        art_count = self.artwork_cache.cached_count() if hasattr(self.artwork_cache, "cached_count") else 0

        info_box = tk.Frame(card, bg=THEME["bg_panel"], padx=10, pady=8)
        info_box.pack(fill=tk.X, pady=(0, 10))

        lbl_db_path = tk.Label(
            info_box,
            text=f"Database file:  {db_path}",
            bg=THEME["bg_panel"],
            fg=THEME["text_secondary"],
            font=("Segoe UI", 8),
            anchor="w",
        )
        lbl_db_path.pack(fill=tk.X)

        lbl_stats = tk.Label(
            info_box,
            text=f"Indexed tracks:  {tracks_count:,} tracks    |    Cached covers:  {art_count} files",
            bg=THEME["bg_panel"],
            fg=THEME["text_muted"],
            font=("Segoe UI", 8),
            anchor="w",
        )
        lbl_stats.pack(fill=tk.X, pady=(4, 0))
        self.lbl_stats = lbl_stats

        # Action buttons
        btn_box = tk.Frame(card, bg=THEME["bg_card"])
        btn_box.pack(fill=tk.X)

        btn_resync = tk.Button(
            btn_box,
            text="↻   Force Resync Library Database",
            command=self._handle_resync,
            bg=THEME["bg_panel"],
            fg=THEME["text_primary"],
            activebackground=THEME["border"],
            activeforeground="#FFFFFF",
            font=("Segoe UI", 8, "bold"),
            relief=tk.FLAT,
            padx=10,
            pady=6,
            cursor="hand2",
        )
        btn_resync.pack(side=tk.LEFT, padx=(0, 8))

        btn_clear_art = tk.Button(
            btn_box,
            text="🗑   Clear Artwork Cache",
            command=self._handle_clear_artwork,
            bg=THEME["bg_panel"],
            fg=THEME["text_primary"],
            activebackground=THEME["border"],
            activeforeground="#FFFFFF",
            font=("Segoe UI", 8),
            relief=tk.FLAT,
            padx=10,
            pady=6,
            cursor="hand2",
        )
        btn_clear_art.pack(side=tk.LEFT)

    def _handle_resync(self) -> None:
        if self.on_resync_db:
            self.on_resync_db()
            messagebox.showinfo("Library Sync", "Initiated library sync with player. Tracks will update shortly.", parent=self)

    def _handle_clear_artwork(self) -> None:
        if hasattr(self.artwork_cache, "clear_cache"):
            self.artwork_cache.clear_cache()
            if hasattr(self, "lbl_stats"):
                tracks_count = self.library_db.get_track_count()
                self.lbl_stats.config(text=f"Indexed tracks:  {tracks_count:,} tracks    |    Cached covers:  0 files")
            messagebox.showinfo("Artwork Cache", "Artwork cache cleared. Images will re-download smoothly as you browse.", parent=self)

    # -------------------------------------------------------------------------
    # 5. Device Connection Section
    # -------------------------------------------------------------------------
    def _build_connection_section(self, parent: tk.Widget) -> None:
        card = self._make_card(
            parent,
            "📡   Network & Device Connection",
            "Target player IP address and discovery configuration.",
        )

        host_box = tk.Frame(card, bg=THEME["bg_panel"], padx=10, pady=8)
        host_box.pack(fill=tk.X, pady=(0, 10))

        lbl_conn = tk.Label(
            host_box,
            text=f"Connected Player:  {self.client.host}:{self.client.port}  (TempoTec Variations V1)",
            bg=THEME["bg_panel"],
            fg=THEME["text_primary"],
            font=("Segoe UI", 9, "bold"),
            anchor="w",
        )
        lbl_conn.pack(fill=tk.X)

        btn_row = tk.Frame(card, bg=THEME["bg_card"])
        btn_row.pack(fill=tk.X)

        btn_scan = tk.Button(
            btn_row,
            text="🔍   Scan Network / Connect to Device",
            command=self._handle_open_connect,
            bg=THEME["bg_panel"],
            fg=self.theme_mgr.accent_color,
            activebackground=THEME["border"],
            activeforeground="#FFFFFF",
            font=("Segoe UI", 8, "bold"),
            relief=tk.FLAT,
            padx=10,
            pady=6,
            cursor="hand2",
        )
        btn_scan.pack(side=tk.LEFT)

    def _handle_open_connect(self) -> None:
        if self.on_open_connect:
            self.on_open_connect()

    # -------------------------------------------------------------------------
    # 6. About Section
    # -------------------------------------------------------------------------
    def _build_about_section(self, parent: tk.Widget) -> None:
        card = self._make_card(
            parent,
            "ℹ️   About SonixLink",
            "Windows 64-bit Desktop companion for SonixLink / HiByLink players.",
        )

        lbl_info = tk.Label(
            card,
            text=(
                "SonixLink Desktop for Windows (x64) by qirke\n"
                "Compatible with TempoTec Variations V1 and SonixLink / HiByLink devices.\n"
                "Features complete audio playback control, SQLite library browser,\n"
                "hardware DSP controls (MSEB / EQ / PEQ), artwork cache, and real-time syncing."
            ),
            bg=THEME["bg_card"],
            fg=THEME["text_secondary"],
            font=("Segoe UI", 8),
            justify=tk.LEFT,
            anchor="w",
        )
        lbl_info.pack(fill=tk.X, pady=(0, 4))
