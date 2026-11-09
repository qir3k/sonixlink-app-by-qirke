"""
SonixLink Windows Desktop - Main Application Window
Modern, polished 3-Panel desktop UI matching Android SonixLink & Material Design:
  - Official SonixLink app icon on window, taskbar, and header
  - Left Panel: Brand with icon, Connection card, Search, Grouped library navigation, DSP launcher
  - Center Panel: Header with track count, Play All / Shuffle buttons, Alternating striped table
  - Right Panel: Rounded album art, Metadata, Format badge, Seekbar, Controls, Illuminated DSP pills, Volume
"""

from __future__ import annotations

import os
import queue
import threading
import time
import tkinter as tk
from tkinter import messagebox, ttk
from typing import Any, Callable, Dict, List, Optional
from PIL import Image, ImageTk

from app_settings import settings
from artwork_cache import ArtworkCache
from discovery import DeviceBeacon, DiscoveryListener
from dsp_dialog import DspDialog
from library_db import LibraryDatabase
from settings_dialog import SettingsDialog
from sonixlink_client import PlayerInfo, PlayerState, SonixLinkClient
from ui_theme import THEME, DspPillWidget, ThemeManager, format_duration


class MainWindow(tk.Tk):
    """Main window for SonixLink Windows 64-bit Desktop app."""

    def __init__(self, host: Optional[str] = None, port: Optional[int] = None):
        super().__init__()
        self.title("SonixLink - Remote Player for Windows")

        # Set official app icon on window & taskbar
        self._set_app_icon()

        # Start cleanly maximized on Windows so everything fits without manual resizing
        self._setup_window_geometry()

        self.configure(bg=THEME["bg_base"])

        # Determine target host: CLI arg > saved last_host > 192.168.0.17 (default player IP)
        target_host = host or settings.last_host or "192.168.0.17"
        target_port = port or settings.last_port or 7800

        # Core components
        self.theme_mgr = ThemeManager(self)
        self.theme_mgr.set_accent(settings.effective_accent)
        self.theme_mgr.add_accent_listener(lambda _: self._on_accent_updated())
        self.client = SonixLinkClient(host=target_host, port=target_port)
        self.library_db = LibraryDatabase()
        self.artwork_cache = ArtworkCache()
        self.discovery = DiscoveryListener(on_device_found=self._on_device_discovered)

        # Keyboard shortcuts
        self.bind("<Control-comma>", lambda e: self._open_settings_dialog())

        # Runtime State
        self.current_state: Optional[PlayerState] = None
        self.current_nav: str = "tracks"
        self.is_seeking: bool = False
        self.current_art_path: Optional[str] = None
        self.active_folder_path: str = "/"
        self.discovered_devices: Dict[str, DeviceBeacon] = {}
        self.ui_queue: queue.Queue[Callable[[], None]] = queue.Queue()
        self.polling_active: bool = True
        self.is_syncing_db: bool = False
        self._current_photo_ref: Optional[Any] = None
        self._brand_logo_photo: Optional[Any] = None
        self._active_connect_dialog: Optional[tk.Toplevel] = None
        self._active_listbox: Optional[tk.Listbox] = None

        # Build UI
        self.theme_mgr.apply_ttk_styles()
        self._build_ui()

        # Start background services
        self.discovery.start()
        self._start_state_poller()
        self._process_ui_queue()

        # Load local database if available
        self._refresh_library_view()

        # Immediate background connection & automatic database sync check
        threading.Thread(target=self._check_and_auto_sync, daemon=True).start()

    def _set_app_icon(self) -> None:
        """Load and set the official SonixLink icon on the titlebar and Windows taskbar."""
        desktop_dir = os.path.dirname(os.path.abspath(__file__))
        png_path = os.path.join(desktop_dir, "icon.png")
        ico_path = os.path.join(desktop_dir, "icon.ico")

        if os.path.exists(png_path):
            try:
                self._app_icon_photo = tk.PhotoImage(file=png_path)
                self.iconphoto(True, self._app_icon_photo)
            except Exception:
                pass

        if os.path.exists(ico_path):
            try:
                self.iconbitmap(ico_path)
            except Exception:
                pass

    def _setup_window_geometry(self) -> None:
        """Set minimum window bounds and start cleanly maximized on Windows."""
        self.minsize(960, 580)
        try:
            self.state("zoomed")  # Seamless native maximization on Windows
        except Exception:
            sw = self.winfo_screenwidth()
            sh = self.winfo_screenheight()
            w = min(1280, max(980, sw - 60))
            h = min(740, max(620, sh - 80))
            x = max(10, (sw - w) // 2)
            y = max(10, (sh - h) // 2)
            self.geometry(f"{w}x{h}+{x}+{y}")

    # -------------------------------------------------------------------------
    # UI Building
    # -------------------------------------------------------------------------

    def _build_ui(self) -> None:
        self.container = tk.Frame(self, bg=THEME["bg_base"])
        self.container.pack(fill=tk.BOTH, expand=True)

        self._build_left_sidebar(self.container)
        self._build_center_panel(self.container)
        self._build_right_sidebar(self.container)

    def _build_left_sidebar(self, parent: tk.Widget) -> None:
        """Left panel: Brand with Logo, Connection card, Search, Grouped Nav, DSP button."""
        self.left_frame = tk.Frame(parent, bg=THEME["bg_panel"], width=230)
        self.left_frame.pack(side=tk.LEFT, fill=tk.Y)
        self.left_frame.pack_propagate(False)

        # 1. Branding Header with Official App Icon
        brand_frame = tk.Frame(self.left_frame, bg=THEME["bg_panel"])
        brand_frame.pack(fill=tk.X, padx=14, pady=(14, 12))

        # Load 32x32 SonixLink Icon
        desktop_dir = os.path.dirname(os.path.abspath(__file__))
        png_path = os.path.join(desktop_dir, "icon.png")
        if os.path.exists(png_path):
            try:
                im = Image.open(png_path).resize((32, 32), Image.Resampling.LANCZOS)
                self._brand_logo_photo = ImageTk.PhotoImage(im)
                logo_lbl = tk.Label(brand_frame, image=self._brand_logo_photo, bg=THEME["bg_panel"])
                logo_lbl.pack(side=tk.LEFT, padx=(0, 10))
            except Exception:
                pass

        brand_text_box = tk.Frame(brand_frame, bg=THEME["bg_panel"])
        brand_text_box.pack(side=tk.LEFT, fill=tk.X, expand=True)

        title_lbl = tk.Label(
            brand_text_box,
            text="SonixLink",
            bg=THEME["bg_panel"],
            fg=THEME["text_primary"],
            font=("Segoe UI", 15, "bold"),
            anchor="w",
        )
        title_lbl.pack(fill=tk.X)

        self.lbl_desktop_badge = tk.Label(
            brand_text_box,
            text="TempoTec Variations V1",
            bg=THEME["bg_panel"],
            fg=self.theme_mgr.accent_color,
            font=("Segoe UI", 8, "bold"),
            anchor="w",
        )
        self.lbl_desktop_badge.pack(fill=tk.X)

        # 2. Modern Connection Card
        self.dev_card = tk.Frame(
            self.left_frame,
            bg=THEME["bg_card"],
            padx=12,
            pady=10,
            highlightbackground=THEME["border"],
            highlightthickness=1,
        )
        self.dev_card.pack(fill=tk.X, padx=12, pady=(0, 12))

        top_row = tk.Frame(self.dev_card, bg=THEME["bg_card"])
        top_row.pack(fill=tk.X)

        self.lbl_status_dot = tk.Label(
            top_row,
            text="●",
            bg=THEME["bg_card"],
            fg=THEME["status_red"],
            font=("Segoe UI", 11),
        )
        self.lbl_status_dot.pack(side=tk.LEFT, padx=(0, 5))

        self.lbl_dev_name = tk.Label(
            top_row,
            text="TempoTec V1",
            bg=THEME["bg_card"],
            fg=THEME["text_primary"],
            font=("Segoe UI", 9, "bold"),
            anchor="w",
        )
        self.lbl_dev_name.pack(side=tk.LEFT, fill=tk.X, expand=True)

        self.lbl_battery = tk.Label(
            top_row,
            text="",
            bg=THEME["bg_card"],
            fg=THEME["text_secondary"],
            font=("Segoe UI", 8, "bold"),
        )
        self.lbl_battery.pack(side=tk.RIGHT)

        self.lbl_dev_host = tk.Label(
            self.dev_card,
            text=f"{self.client.host}:{self.client.port}",
            bg=THEME["bg_card"],
            fg=THEME["text_muted"],
            font=("Segoe UI", 8),
            anchor="w",
        )
        self.lbl_dev_host.pack(fill=tk.X, pady=(2, 6))

        btn_row = tk.Frame(self.dev_card, bg=THEME["bg_card"])
        btn_row.pack(fill=tk.X)

        self.btn_connect = tk.Button(
            btn_row,
            text="🔍  Devices",
            command=self._show_connect_dialog,
            bg=THEME["bg_field"],
            fg=THEME["text_primary"],
            activebackground=THEME["border_light"],
            activeforeground="#FFFFFF",
            font=("Segoe UI", 8),
            relief=tk.FLAT,
            padx=8,
            pady=3,
            cursor="hand2",
        )
        self.btn_connect.pack(side=tk.LEFT, padx=(0, 6))

        self.btn_sync_db = tk.Button(
            btn_row,
            text="↻  Sync DB",
            command=self._sync_database_manual,
            bg=THEME["bg_field"],
            fg=THEME["text_primary"],
            activebackground=THEME["border_light"],
            activeforeground="#FFFFFF",
            font=("Segoe UI", 8),
            relief=tk.FLAT,
            padx=8,
            pady=3,
            cursor="hand2",
        )
        self.btn_sync_db.pack(side=tk.LEFT)

        # 3. Fast Search Box
        search_frame = tk.Frame(self.left_frame, bg=THEME["bg_panel"])
        search_frame.pack(fill=tk.X, padx=12, pady=(0, 10))

        self.search_var = tk.StringVar()
        self.search_entry = tk.Entry(
            search_frame,
            textvariable=self.search_var,
            bg=THEME["bg_field"],
            fg=THEME["text_primary"],
            insertbackground=THEME["text_primary"],
            font=("Segoe UI", 9),
            relief=tk.FLAT,
            highlightbackground=THEME["border"],
            highlightthickness=1,
        )
        self.search_entry.pack(fill=tk.X, ipady=4)
        self.search_entry.insert(0, "Search library...")
        self.search_entry.bind("<FocusIn>", self._on_search_focus_in)
        self.search_entry.bind("<FocusOut>", self._on_search_focus_out)
        self.search_entry.bind("<KeyRelease>", self._on_search_change)

        # 4. Grouped Navigation Menu
        nav_container = tk.Frame(self.left_frame, bg=THEME["bg_panel"])
        nav_container.pack(fill=tk.BOTH, expand=True, padx=8)

        self.nav_buttons: Dict[str, tk.Button] = {}

        fav_icon = "⭐" if settings.effective_fav_mark == "star" else "♥"
        nav_groups = [
            ("LIBRARY", [
                ("tracks", "🎵  All Tracks"),
                ("albums", "💿  Albums"),
                ("artists", "👤  Artists"),
                ("genres", "🏷️  Genres"),
                ("years", "📅  Years"),
            ]),
            ("COLLECTION", [
                ("favorites", f"{fav_icon}  Favourites"),
                ("playlists", "📋  Playlists"),
                ("queue", "📜  Play Queue"),
            ]),
            ("STORAGE", [
                ("files", "📁  SD Card Files"),
                ("streaming", "🌐  Streaming"),
            ]),
        ]

        for group_title, items in nav_groups:
            grp_lbl = tk.Label(
                nav_container,
                text=group_title,
                bg=THEME["bg_panel"],
                fg=THEME["text_muted"],
                font=("Segoe UI", 7, "bold"),
                anchor="w",
            )
            grp_lbl.pack(fill=tk.X, padx=8, pady=(8, 2))

            for key, label in items:
                btn = tk.Button(
                    nav_container,
                    text=label,
                    anchor="w",
                    bg=THEME["bg_panel"],
                    fg=THEME["text_secondary"],
                    activebackground=THEME["bg_card"],
                    activeforeground=THEME["text_primary"],
                    font=("Segoe UI", 9),
                    relief=tk.FLAT,
                    padx=10,
                    pady=3,
                    cursor="hand2",
                    command=lambda k=key: self._on_nav_select(k),
                )
                btn.pack(fill=tk.X, pady=1)
                self.nav_buttons[key] = btn

        self._highlight_nav("tracks")

        # 5. Bottom Settings & Hardware DSP Buttons
        bottom_box = tk.Frame(self.left_frame, bg=THEME["bg_panel"])
        bottom_box.pack(fill=tk.X, side=tk.BOTTOM, padx=12, pady=12)

        self.btn_settings = tk.Button(
            bottom_box,
            text="⚙️  Settings  ›",
            command=self._open_settings_dialog,
            bg=THEME["bg_card"],
            fg=THEME["text_primary"],
            activebackground=THEME["border_light"],
            activeforeground="#FFFFFF",
            font=("Segoe UI", 9, "bold"),
            relief=tk.FLAT,
            padx=8,
            pady=7,
            cursor="hand2",
        )
        self.btn_settings.pack(fill=tk.X, pady=(0, 6))

        self.btn_dsp_dialog = tk.Button(
            bottom_box,
            text="🎛️  Audio & DSP Settings  ›",
            command=self._open_dsp_dialog,
            bg=THEME["bg_card"],
            fg=THEME["text_primary"],
            activebackground=THEME["border_light"],
            activeforeground="#FFFFFF",
            font=("Segoe UI", 9, "bold"),
            relief=tk.FLAT,
            padx=8,
            pady=7,
            cursor="hand2",
        )
        self.btn_dsp_dialog.pack(fill=tk.X)

    def _build_center_panel(self, parent: tk.Widget) -> None:
        """Center panel: Modern header, action buttons, alternating striped table."""
        self.center_frame = tk.Frame(parent, bg=THEME["bg_base"])
        self.center_frame.pack(side=tk.LEFT, fill=tk.BOTH, expand=True)

        # 1. Header Bar
        self.center_header = tk.Frame(self.center_frame, bg=THEME["bg_base"])
        self.center_header.pack(fill=tk.X, padx=18, pady=(14, 8))

        self.lbl_view_title = tk.Label(
            self.center_header,
            text="All Tracks",
            bg=THEME["bg_base"],
            fg=THEME["text_primary"],
            font=("Segoe UI", 16, "bold"),
            anchor="w",
        )
        self.lbl_view_title.pack(side=tk.LEFT)

        self.lbl_view_count = tk.Label(
            self.center_header,
            text="0 tracks",
            bg=THEME["bg_card"],
            fg=THEME["text_muted"],
            font=("Segoe UI", 8, "bold"),
            padx=8,
            pady=2,
        )
        self.lbl_view_count.pack(side=tk.LEFT, padx=(12, 0))

        # Quick action buttons on header right
        self.btn_play_all = tk.Button(
            self.center_header,
            text="▶  Play All",
            command=self._on_play_all_clicked,
            bg=THEME["bg_card"],
            fg=THEME["text_primary"],
            activebackground=THEME["bg_card_hover"],
            activeforeground="#FFFFFF",
            font=("Segoe UI", 8, "bold"),
            relief=tk.FLAT,
            padx=10,
            pady=3,
            cursor="hand2",
        )
        self.btn_play_all.pack(side=tk.RIGHT, padx=(6, 0))

        self.btn_shuffle_all = tk.Button(
            self.center_header,
            text="🔀  Shuffle",
            command=self._on_shuffle_all_clicked,
            bg=THEME["bg_card"],
            fg=THEME["text_primary"],
            activebackground=THEME["bg_card_hover"],
            activeforeground="#FFFFFF",
            font=("Segoe UI", 8, "bold"),
            relief=tk.FLAT,
            padx=10,
            pady=3,
            cursor="hand2",
        )
        self.btn_shuffle_all.pack(side=tk.RIGHT)

        # Breadcrumb / back button for folder navigation
        self.btn_nav_back = tk.Button(
            self.center_header,
            text="⬆ Up Folder",
            command=self._on_folder_up,
            bg=THEME["bg_card"],
            fg=THEME["text_primary"],
            relief=tk.FLAT,
            font=("Segoe UI", 8),
            padx=8,
            pady=3,
            cursor="hand2",
        )

        # Sync Banner (auto-appears when downloading index)
        self.sync_banner = tk.Frame(self.center_frame, bg=THEME["bg_card"], padx=14, pady=6)
        self.lbl_sync_msg = tk.Label(
            self.sync_banner,
            text="⚡ Syncing library index from SonixPlayer...",
            bg=THEME["bg_card"],
            fg=self.theme_mgr.accent_color,
            font=("Segoe UI", 8, "bold"),
        )
        self.lbl_sync_msg.pack(side=tk.LEFT)

        # 2. Modern Table Treeview Container
        table_container = tk.Frame(self.center_frame, bg=THEME["bg_base"])
        table_container.pack(fill=tk.BOTH, expand=True, padx=16, pady=(0, 14))

        self.tree_scroll_y = ttk.Scrollbar(table_container, orient=tk.VERTICAL)
        self.tree = ttk.Treeview(
            table_container,
            selectmode="browse",
            show="headings",
            yscrollcommand=self.tree_scroll_y.set,
        )
        self.tree_scroll_y.config(command=self.tree.yview)

        self.tree_scroll_y.pack(side=tk.RIGHT, fill=tk.Y)
        self.tree.pack(side=tk.LEFT, fill=tk.BOTH, expand=True)

        # Subtle zebra striping for clean readability
        self.tree.tag_configure("evenrow", background=THEME["row_even"])
        self.tree.tag_configure("oddrow", background=THEME["row_odd"])

        self.tree.bind("<Double-1>", self._on_tree_double_click)
        self.tree.bind("<Button-3>", self._on_tree_right_click)

        # Context Menu
        self.context_menu = tk.Menu(
            self,
            tearoff=0,
            bg=THEME["bg_card"],
            fg=THEME["text_primary"],
            activebackground=self.theme_mgr.accent_color,
        )
        self.context_menu.add_command(label="Play Now", command=self._menu_play_now)
        self.context_menu.add_command(label="Play Next", command=self._menu_play_next)
        self.context_menu.add_command(label="Add to Queue", command=self._menu_enqueue)
        self.context_menu.add_separator()
        self.context_menu.add_command(label="Toggle Favourite", command=self._menu_toggle_fav)
        self.context_menu.add_command(label="Copy File Path", command=self._menu_copy_path)

        self.current_rows: List[Dict[str, Any]] = []

    def _build_right_sidebar(self, parent: tk.Widget) -> None:
        """Right panel: Rounded artwork, Metadata, Format badge, Seekbar, Controls, DSP pills, Volume."""
        self.right_frame = tk.Frame(parent, bg=THEME["bg_panel"], width=290)
        self.right_frame.pack(side=tk.RIGHT, fill=tk.Y)
        self.right_frame.pack_propagate(False)

        right_content = tk.Frame(self.right_frame, bg=THEME["bg_panel"], padx=16, pady=16)
        right_content.pack(fill=tk.BOTH, expand=True)

        # 1. Album Artwork Canvas (220x220 with rounded mask)
        self.art_canvas = tk.Canvas(
            right_content,
            width=220,
            height=220,
            bg=THEME["bg_card"],
            highlightbackground=THEME["border"],
            highlightthickness=1,
            cursor="hand2",
        )
        self.art_canvas.pack(pady=(0, 12))
        self.art_canvas.bind("<Button-1>", lambda e: self._refresh_artwork(force=True))
        self._draw_art_placeholder()

        # 2. Track Metadata
        self.lbl_track_title = tk.Label(
            right_content,
            text="No Track Playing",
            bg=THEME["bg_panel"],
            fg=THEME["text_primary"],
            font=("Segoe UI", 12, "bold"),
            wraplength=260,
            justify=tk.CENTER,
        )
        self.lbl_track_title.pack(fill=tk.X)

        self.lbl_track_artist = tk.Label(
            right_content,
            text="TempoTec Variations V1",
            bg=THEME["bg_panel"],
            fg=THEME["text_secondary"],
            font=("Segoe UI", 9),
            wraplength=260,
            justify=tk.CENTER,
        )
        self.lbl_track_artist.pack(fill=tk.X, pady=(2, 0))

        # Audio Format Pill Badge (e.g. "FLAC • 44.1kHz • 24bit")
        self.lbl_audio_badge = tk.Label(
            right_content,
            text="",
            bg=THEME["bg_card"],
            fg=self.theme_mgr.accent_color,
            font=("Segoe UI", 8, "bold"),
            padx=10,
            pady=2,
            justify=tk.CENTER,
        )
        self.lbl_audio_badge.pack(pady=(4, 10))

        # 3. Seek Bar & Time Labels
        seek_frame = tk.Frame(right_content, bg=THEME["bg_panel"])
        seek_frame.pack(fill=tk.X, pady=(0, 8))

        self.seek_var = tk.DoubleVar(value=0.0)
        self.seek_slider = ttk.Scale(
            seek_frame,
            from_=0,
            to=100,
            orient=tk.HORIZONTAL,
            variable=self.seek_var,
            command=self._on_seek_drag,
        )
        self.seek_slider.pack(fill=tk.X)
        self.seek_slider.bind("<ButtonPress-1>", self._on_seek_start)
        self.seek_slider.bind("<ButtonRelease-1>", self._on_seek_end)

        time_lbl_frame = tk.Frame(seek_frame, bg=THEME["bg_panel"])
        time_lbl_frame.pack(fill=tk.X, pady=(3, 0))

        self.lbl_time_elapsed = tk.Label(
            time_lbl_frame,
            text="0:00",
            bg=THEME["bg_panel"],
            fg=THEME["text_muted"],
            font=("Segoe UI", 8),
        )
        self.lbl_time_elapsed.pack(side=tk.LEFT)

        self.lbl_queue_pos = tk.Label(
            time_lbl_frame,
            text="",
            bg=THEME["bg_panel"],
            fg=THEME["text_muted"],
            font=("Segoe UI", 8),
        )
        self.lbl_queue_pos.pack(side=tk.LEFT, expand=True)

        self.lbl_time_total = tk.Label(
            time_lbl_frame,
            text="0:00",
            bg=THEME["bg_panel"],
            fg=THEME["text_muted"],
            font=("Segoe UI", 8),
        )
        self.lbl_time_total.pack(side=tk.RIGHT)

        # 4. Modern Playback Controls Row
        ctrl_frame = tk.Frame(right_content, bg=THEME["bg_panel"])
        ctrl_frame.pack(fill=tk.X, pady=(0, 14))

        self.btn_mode = tk.Button(
            ctrl_frame,
            text="🔁",
            command=self._on_toggle_mode,
            bg=THEME["bg_panel"],
            fg=THEME["text_secondary"],
            activebackground=THEME["bg_card"],
            relief=tk.FLAT,
            font=("Segoe UI", 12),
            cursor="hand2",
            width=2,
        )
        self.btn_mode.pack(side=tk.LEFT)

        self.btn_prev = tk.Button(
            ctrl_frame,
            text="⏮",
            command=lambda: self._async_cmd(self.client.prev),
            bg=THEME["bg_panel"],
            fg=THEME["text_primary"],
            activebackground=THEME["bg_card"],
            relief=tk.FLAT,
            font=("Segoe UI", 14),
            cursor="hand2",
            width=3,
        )
        self.btn_prev.pack(side=tk.LEFT, padx=3)

        # Circular Accent Play/Pause button
        self.btn_play = tk.Button(
            ctrl_frame,
            text="▶",
            command=self._on_toggle_play,
            bg=self.theme_mgr.accent_color,
            fg="#FFFFFF",
            activebackground=self.theme_mgr.accent_color,
            activeforeground="#FFFFFF",
            relief=tk.FLAT,
            font=("Segoe UI", 16, "bold"),
            cursor="hand2",
            width=4,
            height=1,
        )
        self.btn_play.pack(side=tk.LEFT, padx=6)

        self.btn_next = tk.Button(
            ctrl_frame,
            text="⏭",
            command=lambda: self._async_cmd(self.client.next),
            bg=THEME["bg_panel"],
            fg=THEME["text_primary"],
            activebackground=THEME["bg_card"],
            relief=tk.FLAT,
            font=("Segoe UI", 14),
            cursor="hand2",
            width=3,
        )
        self.btn_next.pack(side=tk.LEFT, padx=3)

        _, outline = self._get_fav_glyphs()
        self.btn_fav = tk.Button(
            ctrl_frame,
            text=outline,
            command=self._on_toggle_fav_current,
            bg=THEME["bg_panel"],
            fg=THEME["text_secondary"],
            activebackground=THEME["bg_card"],
            relief=tk.FLAT,
            font=("Segoe UI", 14),
            cursor="hand2",
            width=2,
        )
        self.btn_fav.pack(side=tk.LEFT)

        # 5. Illuminated DSP Quick-Pills (MSEB, EQ, PEQ, Profile 1/2)
        dsp_box = tk.Frame(
            right_content,
            bg=THEME["bg_card"],
            padx=10,
            pady=8,
            highlightbackground=THEME["border"],
            highlightthickness=1,
        )
        dsp_box.pack(fill=tk.X, pady=(0, 14))

        dsp_hdr = tk.Frame(dsp_box, bg=THEME["bg_card"])
        dsp_hdr.pack(fill=tk.X, pady=(0, 6))

        dsp_lbl = tk.Label(
            dsp_hdr,
            text="HARDWARE DSP",
            bg=THEME["bg_card"],
            fg=THEME["text_muted"],
            font=("Segoe UI", 7, "bold"),
        )
        dsp_lbl.pack(side=tk.LEFT)

        self.dsp_tune_btn = tk.Button(
            dsp_hdr,
            text="Tune ⚙",
            command=self._open_dsp_dialog,
            bg=THEME["bg_card"],
            fg=self.theme_mgr.accent_color,
            relief=tk.FLAT,
            font=("Segoe UI", 8, "bold"),
            cursor="hand2",
        )
        self.dsp_tune_btn.pack(side=tk.RIGHT)

        pills_row = tk.Frame(dsp_box, bg=THEME["bg_card"])
        pills_row.pack(fill=tk.X)

        self.pill_mseb = DspPillWidget(
            pills_row,
            label="MSEB",
            active=False,
            theme_mgr=self.theme_mgr,
            on_click=self._on_click_pill_mseb,
            width=56,
            height=26,
        )
        self.pill_mseb.pack(side=tk.LEFT, padx=(0, 4))

        self.pill_eq = DspPillWidget(
            pills_row,
            label="EQ",
            active=False,
            theme_mgr=self.theme_mgr,
            on_click=self._on_click_pill_eq,
            width=50,
            height=26,
        )
        self.pill_eq.pack(side=tk.LEFT, padx=(0, 4))

        self.pill_peq = DspPillWidget(
            pills_row,
            label="PEQ",
            active=False,
            theme_mgr=self.theme_mgr,
            on_click=self._on_click_pill_peq,
            width=54,
            height=26,
        )
        self.pill_peq.pack(side=tk.LEFT, padx=(0, 4))

        self.pill_peq_profile = DspPillWidget(
            pills_row,
            label="PEQ P1",
            active=False,
            theme_mgr=self.theme_mgr,
            on_click=self._on_click_pill_peq_profile,
            width=68,
            height=26,
        )
        self.pill_peq_profile.pack(side=tk.LEFT)

        # 6. Volume Control Row
        vol_frame = tk.Frame(right_content, bg=THEME["bg_panel"])
        vol_frame.pack(fill=tk.X)

        self.btn_mute = tk.Button(
            vol_frame,
            text="🔊",
            command=self._on_toggle_mute,
            bg=THEME["bg_panel"],
            fg=THEME["text_primary"],
            relief=tk.FLAT,
            font=("Segoe UI", 10),
            cursor="hand2",
            width=2,
        )
        self.btn_mute.pack(side=tk.LEFT, padx=(0, 6))

        self.vol_var = tk.DoubleVar(value=50.0)
        self.vol_slider = ttk.Scale(
            vol_frame,
            from_=0,
            to=100,
            orient=tk.HORIZONTAL,
            variable=self.vol_var,
            command=self._on_volume_change,
        )
        self.vol_slider.pack(side=tk.LEFT, fill=tk.X, expand=True)

        self.lbl_vol_val = tk.Label(
            vol_frame,
            text="50%",
            bg=THEME["bg_panel"],
            fg=THEME["text_muted"],
            font=("Segoe UI", 8, "bold"),
            width=4,
            anchor="e",
        )
        self.lbl_vol_val.pack(side=tk.RIGHT, padx=(4, 0))

    # -------------------------------------------------------------------------
    # Auto-Discovery & Auto-Sync
    # -------------------------------------------------------------------------

    def _on_device_discovered(self, beacon: DeviceBeacon) -> None:
        """Called whenever a SonixPlayer is seen via UDP or network probe."""
        self.discovered_devices[beacon.ip] = beacon

        # If dialog is open, update its listbox live
        if self._active_listbox and self._active_listbox.winfo_exists():
            cur_items = self._active_listbox.get(0, tk.END)
            entry = f"{beacon.name} ({beacon.ip}:{beacon.port})"
            if entry not in cur_items:
                self._active_listbox.insert(tk.END, entry)

        # Auto-connect if not connected or targeting localhost
        if not self.client.is_connected or self.client.host == "127.0.0.1":
            self.client.set_target(beacon.ip, beacon.port)
            settings.last_host = beacon.ip
            settings.last_port = beacon.port
            self.ui_queue.put(lambda: self._on_device_connected(beacon.name, beacon.ip, beacon.port))
            threading.Thread(target=self._check_and_auto_sync, daemon=True).start()

    def _on_device_connected(self, name: str, host: str, port: int) -> None:
        self.lbl_dev_name.config(text=name)
        self.lbl_dev_host.config(text=f"{host}:{port}")
        self.lbl_status_dot.config(fg=THEME["status_green"])

    def _check_and_auto_sync(self) -> None:
        """Check player info and auto-download SQLite index if stale or missing."""
        if self.is_syncing_db:
            return

        info = self.client.get_info()
        if not info:
            return

        self.ui_queue.put(lambda: self._on_device_connected(info.name, self.client.host, self.client.port))

        needs_sync = False
        if not self.library_db.is_available():
            needs_sync = True
        elif info.stamp and info.stamp != settings.db_stamp:
            needs_sync = True

        if needs_sync and info.db_available and not info.scanning:
            self.is_syncing_db = True
            self.ui_queue.put(lambda: self._show_sync_banner(True, f"⚡ Auto-syncing library index ({info.tracks:,} tracks)..."))

            dest_path = self.library_db.db_path
            ok = self.client.download_database_to_file(dest_path)
            if ok:
                settings.db_stamp = info.stamp
                self.library_db.reload()
                self.ui_queue.put(self._on_auto_sync_success)
            else:
                self.ui_queue.put(lambda: self._show_sync_banner(False))
            self.is_syncing_db = False

    def _show_sync_banner(self, visible: bool, msg: str = "") -> None:
        if visible:
            self.lbl_sync_msg.config(text=msg)
            self.sync_banner.pack(fill=tk.X, padx=16, pady=(0, 6), before=self.center_frame.winfo_children()[1])
        else:
            self.sync_banner.pack_forget()

    def _on_auto_sync_success(self) -> None:
        self._show_sync_banner(False)
        self._refresh_library_view()

    def _sync_database_manual(self) -> None:
        self.btn_sync_db.config(text="Syncing...", state=tk.DISABLED)

        def worker():
            info = self.client.get_info()
            dest_path = self.library_db.db_path
            ok = self.client.download_database_to_file(dest_path)
            if ok:
                if info:
                    settings.db_stamp = info.stamp
                self.library_db.reload()
                self.ui_queue.put(lambda: self._on_manual_sync_finished(True))
            else:
                self.ui_queue.put(lambda: self._on_manual_sync_finished(False))

        threading.Thread(target=worker, daemon=True).start()

    def _on_manual_sync_finished(self, success: bool) -> None:
        self.btn_sync_db.config(text="↻  Sync DB", state=tk.NORMAL)
        if success:
            self._refresh_library_view()
            messagebox.showinfo("Library Synced", "Successfully synced audio library from SonixPlayer!")
        else:
            messagebox.showwarning("Sync Failed", "Could not download database. Check connection.")

    # -------------------------------------------------------------------------
    # Navigation & Views
    # -------------------------------------------------------------------------

    def _on_nav_select(self, nav_key: str) -> None:
        self.current_nav = nav_key
        self._highlight_nav(nav_key)
        self._refresh_library_view()

    def _highlight_nav(self, active_key: str) -> None:
        for key, btn in self.nav_buttons.items():
            if key == active_key:
                btn.configure(
                    bg=THEME["bg_card"],
                    fg=self.theme_mgr.accent_color,
                    font=("Segoe UI", 9, "bold"),
                )
            else:
                btn.configure(
                    bg=THEME["bg_panel"],
                    fg=THEME["text_secondary"],
                    font=("Segoe UI", 9),
                )

    def _refresh_library_view(self) -> None:
        for item in self.tree.get_children():
            self.tree.delete(item)
        self.current_rows.clear()
        self.btn_nav_back.pack_forget()

        if self.current_nav == "tracks":
            self._render_tracks_view()
        elif self.current_nav == "albums":
            self._render_albums_view()
        elif self.current_nav == "artists":
            self._render_artists_view()
        elif self.current_nav == "genres":
            self._render_genres_view()
        elif self.current_nav == "years":
            self._render_years_view()
        elif self.current_nav == "favorites":
            self._render_favorites_view()
        elif self.current_nav == "playlists":
            self._render_playlists_view()
        elif self.current_nav == "files":
            self._render_files_view(self.active_folder_path)
        elif self.current_nav == "queue":
            self._render_queue_view()
        elif self.current_nav == "streaming":
            self._render_streaming_view()

    def _setup_tree_columns(self, columns: List[tuple[str, str, int, str]]) -> None:
        col_ids = [c[0] for c in columns]
        self.tree["columns"] = col_ids
        for col_id, heading, width, anchor in columns:
            self.tree.heading(col_id, text=heading, anchor=anchor)
            self.tree.column(col_id, width=width, minwidth=35, anchor=anchor)

    def _render_tracks_view(self, tracks: Optional[List[Dict[str, Any]]] = None) -> None:
        self.lbl_view_title.config(text="All Tracks")
        self._setup_tree_columns([
            ("num", "#", 45, "center"),
            ("title", "Title", 340, "w"),
            ("artist", "Artist", 220, "w"),
            ("album", "Album", 220, "w"),
            ("duration", "Time", 70, "center"),
        ])
        if tracks is None:
            tracks = self.library_db.get_all_tracks()

        self.current_rows = tracks
        self.lbl_view_count.config(text=f"{len(tracks):,} tracks")

        for idx, t in enumerate(tracks, start=1):
            dur = t.get("duration", 0)
            dur_str = format_duration(dur)
            tag = "evenrow" if idx % 2 == 0 else "oddrow"
            self.tree.insert(
                "",
                tk.END,
                iid=str(idx - 1),
                tags=(tag,),
                values=(
                    t.get("track_num") or idx,
                    t.get("title") or os.path.basename(t.get("path", "Track")),
                    t.get("artist") or "Unknown Artist",
                    t.get("album") or "Unknown Album",
                    dur_str,
                ),
            )

    def _render_albums_view(self) -> None:
        self.lbl_view_title.config(text="Albums")
        self._setup_tree_columns([
            ("album", "Album", 400, "w"),
            ("artist", "Artist", 280, "w"),
            ("tracks", "Tracks", 80, "center"),
        ])
        albums = self.library_db.get_albums()
        self.current_rows = albums
        self.lbl_view_count.config(text=f"{len(albums):,} albums")

        for idx, a in enumerate(albums):
            tag = "evenrow" if idx % 2 == 0 else "oddrow"
            self.tree.insert(
                "",
                tk.END,
                iid=str(idx),
                tags=(tag,),
                values=(a.get("name") or "Unknown Album", a.get("artist") or "", a.get("track_count") or 1),
            )

    def _render_artists_view(self) -> None:
        self.lbl_view_title.config(text="Artists")
        self._setup_tree_columns([
            ("artist", "Artist Name", 560, "w"),
            ("tracks", "Tracks", 90, "center"),
        ])
        artists = self.library_db.get_artists()
        self.current_rows = artists
        self.lbl_view_count.config(text=f"{len(artists):,} artists")

        for idx, a in enumerate(artists):
            tag = "evenrow" if idx % 2 == 0 else "oddrow"
            self.tree.insert("", tk.END, iid=str(idx), tags=(tag,), values=(a.get("name") or "Unknown Artist", a.get("track_count") or 1))

    def _render_genres_view(self) -> None:
        self.lbl_view_title.config(text="Genres")
        self._setup_tree_columns([
            ("genre", "Genre", 560, "w"),
            ("tracks", "Tracks", 90, "center"),
        ])
        genres = self.library_db.get_genres()
        self.current_rows = genres
        self.lbl_view_count.config(text=f"{len(genres):,} genres")

        for idx, g in enumerate(genres):
            tag = "evenrow" if idx % 2 == 0 else "oddrow"
            self.tree.insert("", tk.END, iid=str(idx), tags=(tag,), values=(g.get("name") or "Unknown", g.get("track_count") or 1))

    def _render_years_view(self) -> None:
        self.lbl_view_title.config(text="Years")
        self._setup_tree_columns([
            ("year", "Release Year", 450, "w"),
            ("tracks", "Tracks", 90, "center"),
        ])
        years = self.library_db.get_years()
        self.current_rows = years
        self.lbl_view_count.config(text=f"{len(years):,} release years")

        for idx, y in enumerate(years):
            tag = "evenrow" if idx % 2 == 0 else "oddrow"
            self.tree.insert("", tk.END, iid=str(idx), tags=(tag,), values=(y.get("name") or "Unknown", y.get("track_count") or 1))

    def _render_favorites_view(self) -> None:
        self.lbl_view_title.config(text="Favourites")
        favs = self.library_db.get_favorites()
        self._render_tracks_view(favs)
        self.lbl_view_title.config(text="Favourites")
        self.lbl_view_count.config(text=f"{len(favs):,} favourites")

    def _render_playlists_view(self) -> None:
        self.lbl_view_title.config(text="Playlists")
        self._setup_tree_columns([
            ("name", "Playlist Name", 520, "w"),
            ("count", "Tracks", 90, "center"),
        ])
        pls = self.library_db.get_playlists()
        self.current_rows = pls
        self.lbl_view_count.config(text=f"{len(pls):,} playlists")

        for idx, p in enumerate(pls):
            tag = "evenrow" if idx % 2 == 0 else "oddrow"
            self.tree.insert("", tk.END, iid=str(idx), tags=(tag,), values=(p.get("name", "Playlist"), p.get("track_count", 0)))

    def _render_files_view(self, folder_path: str = "/") -> None:
        self.active_folder_path = folder_path
        self.lbl_view_title.config(text=f"SD Card: {folder_path}")
        self.btn_nav_back.pack(side=tk.RIGHT)

        self._setup_tree_columns([
            ("type", "Type", 70, "center"),
            ("name", "Name", 600, "w"),
        ])

        def worker():
            res = self.client.browse_directory(folder_path)
            items = res.get("items", []) if res else []
            self.ui_queue.put(lambda: self._populate_files_tree(items))

        threading.Thread(target=worker, daemon=True).start()

    def _populate_files_tree(self, items: List[Dict[str, Any]]) -> None:
        for item in self.tree.get_children():
            self.tree.delete(item)
        self.current_rows = items
        self.lbl_view_count.config(text=f"{len(items):,} items")

        for idx, it in enumerate(items):
            icon = "📁 Dir" if it.get("is_dir") else "🎵 Audio"
            tag = "evenrow" if idx % 2 == 0 else "oddrow"
            self.tree.insert("", tk.END, iid=str(idx), tags=(tag,), values=(icon, it.get("name", "Item")))

    def _on_folder_up(self) -> None:
        parent = os.path.dirname(self.active_folder_path.rstrip("/"))
        if not parent:
            parent = "/"
        self._render_files_view(parent)

    def _render_queue_view(self) -> None:
        self.lbl_view_title.config(text="Play Queue")
        self._setup_tree_columns([
            ("idx", "#", 50, "center"),
            ("name", "Track / Path", 700, "w"),
        ])

        def worker():
            q = self.client.get_queue()
            self.ui_queue.put(lambda: self._populate_queue_tree(q))

        threading.Thread(target=worker, daemon=True).start()

    def _populate_queue_tree(self, queue_items: List[Dict[str, Any]]) -> None:
        for item in self.tree.get_children():
            self.tree.delete(item)
        self.current_rows = queue_items
        self.lbl_view_count.config(text=f"{len(queue_items):,} items in queue")

        cur_idx = self.current_state.queue_index if self.current_state else -1
        for idx, item in enumerate(queue_items):
            marker = f"▶ {idx + 1}" if idx == cur_idx else str(idx + 1)
            name = item.get("title") or item.get("name") or item.get("path") or f"Item {idx + 1}"
            tag = "evenrow" if idx % 2 == 0 else "oddrow"
            self.tree.insert("", tk.END, iid=str(idx), tags=(tag,), values=(marker, name))

    def _render_streaming_view(self) -> None:
        self.lbl_view_title.config(text="Streaming Services")
        self._setup_tree_columns([
            ("service", "Service", 320, "w"),
            ("status", "Status", 200, "center"),
        ])
        services = [
            {"name": "TIDAL", "status": "Available via SonixPlayer"},
            {"name": "Qobuz", "status": "Available via SonixPlayer"},
            {"name": "DLNA / UPnP Renderer", "status": "Active"},
            {"name": "Bluetooth Audio Sink", "status": "Ready"},
        ]
        self.current_rows = services
        self.lbl_view_count.config(text=f"{len(services)} services")
        for idx, s in enumerate(services):
            tag = "evenrow" if idx % 2 == 0 else "oddrow"
            self.tree.insert("", tk.END, iid=str(idx), tags=(tag,), values=(s["name"], s["status"]))

    # -------------------------------------------------------------------------
    # Tree Events & Context Menu
    # -------------------------------------------------------------------------

    def _on_tree_double_click(self, event: tk.Event) -> None:
        selected = self.tree.selection()
        if not selected:
            return
        idx = int(selected[0])
        if idx >= len(self.current_rows):
            return
        row = self.current_rows[idx]

        if self.current_nav in ("tracks", "favorites"):
            path = row.get("path")
            if path:
                self._async_cmd(lambda: self.client.play_path(path))
        elif self.current_nav == "albums":
            album_name = row.get("name")
            if album_name:
                tracks = self.library_db.get_album_tracks(album_name)
                self._render_tracks_view(tracks)
                self.lbl_view_title.config(text=f"Album: {album_name}")
        elif self.current_nav == "artists":
            artist_name = row.get("name")
            if artist_name:
                tracks = self.library_db.get_artist_tracks(artist_name)
                self._render_tracks_view(tracks)
                self.lbl_view_title.config(text=f"Artist: {artist_name}")
        elif self.current_nav == "genres":
            genre_name = row.get("name")
            if genre_name:
                tracks = self.library_db.get_genre_tracks(genre_name)
                self._render_tracks_view(tracks)
                self.lbl_view_title.config(text=f"Genre: {genre_name}")
        elif self.current_nav == "years":
            year_val = row.get("name")
            if year_val:
                tracks = self.library_db.get_year_tracks(year_val)
                self._render_tracks_view(tracks)
                self.lbl_view_title.config(text=f"Year: {year_val}")
        elif self.current_nav == "playlists":
            pl_id = row.get("id")
            if pl_id:
                tracks = self.library_db.get_playlist_tracks(pl_id)
                self._render_tracks_view(tracks)
                self.lbl_view_title.config(text=f"Playlist: {row.get('name')}")
        elif self.current_nav == "files":
            if row.get("is_dir"):
                new_path = row.get("path") or f"{self.active_folder_path.rstrip('/')}/{row.get('name')}"
                self._render_files_view(new_path)
            else:
                path = row.get("path") or f"{self.active_folder_path.rstrip('/')}/{row.get('name')}"
                self._async_cmd(lambda: self.client.play_path(path))
        elif self.current_nav == "queue":
            self._async_cmd(lambda: self.client.play_queue_index(idx))

    def _on_tree_right_click(self, event: tk.Event) -> None:
        item = self.tree.identify_row(event.y)
        if item:
            self.tree.selection_set(item)
            self.context_menu.post(event.x_root, event.y_root)

    def _get_selected_row(self) -> Optional[Dict[str, Any]]:
        sel = self.tree.selection()
        if not sel:
            return None
        idx = int(sel[0])
        return self.current_rows[idx] if idx < len(self.current_rows) else None

    def _menu_play_now(self) -> None:
        row = self._get_selected_row()
        if row and row.get("path"):
            self._async_cmd(lambda: self.client.play_path(row["path"]))

    def _menu_play_next(self) -> None:
        row = self._get_selected_row()
        if row and row.get("path"):
            self._async_cmd(lambda: self.client.enqueue(row["path"], play_next=True))

    def _menu_enqueue(self) -> None:
        row = self._get_selected_row()
        if row and row.get("path"):
            self._async_cmd(lambda: self.client.enqueue(row["path"]))

    def _menu_toggle_fav(self) -> None:
        row = self._get_selected_row()
        if row and row.get("path"):
            self._async_cmd(lambda: self.client.toggle_favorite(row["path"]))

    def _menu_copy_path(self) -> None:
        row = self._get_selected_row()
        if row and row.get("path"):
            self.clipboard_clear()
            self.clipboard_append(row["path"])

    def _on_play_all_clicked(self) -> None:
        if self.current_rows and self.current_rows[0].get("path"):
            self._async_cmd(lambda: self.client.play_path(self.current_rows[0]["path"]))

    def _on_shuffle_all_clicked(self) -> None:
        self._async_cmd(lambda: self.client.play_all(list_type="all", how="shuffle"))

    # -------------------------------------------------------------------------
    # Search
    # -------------------------------------------------------------------------

    def _on_search_focus_in(self, event: tk.Event) -> None:
        if self.search_var.get() == "Search library...":
            self.search_entry.delete(0, tk.END)

    def _on_search_focus_out(self, event: tk.Event) -> None:
        if not self.search_var.get().strip():
            self.search_entry.insert(0, "Search library...")

    def _on_search_change(self, event: tk.Event) -> None:
        q = self.search_var.get().strip()
        if not q or q == "Search library...":
            return
        results = self.library_db.search_tracks(q, limit=150)
        self._render_tracks_view(results)
        self.lbl_view_title.config(text=f"Search: '{q}'")

    # -------------------------------------------------------------------------
    # Playback Controls & DSP Interaction
    # -------------------------------------------------------------------------

    def _on_toggle_play(self) -> None:
        self._async_cmd(self.client.toggle_play_pause)

    def _on_toggle_mode(self) -> None:
        self._async_cmd(self.client.cycle_playback_mode)

    def _on_toggle_fav_current(self) -> None:
        if self.current_state and self.current_state.path:
            self._async_cmd(lambda: self.client.toggle_favorite(self.current_state.path))

    def _on_toggle_mute(self) -> None:
        self._async_cmd(self.client.toggle_mute)

    def _on_volume_change(self, val: str) -> None:
        vol = int(float(val))
        self.lbl_vol_val.config(text=f"{vol}%")
        self._async_cmd(lambda: self.client.set_volume(vol))

    def _on_seek_start(self, event: tk.Event) -> None:
        self.is_seeking = True

    def _on_seek_drag(self, val: str) -> None:
        sec = int(float(val))
        self.lbl_time_elapsed.config(text=format_duration(sec))

    def _on_seek_end(self, event: tk.Event) -> None:
        sec = int(self.seek_var.get())
        self.is_seeking = False
        self._async_cmd(lambda: self.client.seek(sec))

    def _on_click_pill_mseb(self) -> None:
        new_val = not self.pill_mseb.active
        self.pill_mseb.set_active(new_val)
        self._async_cmd(lambda: self.client.set_mseb(new_val))

    def _on_click_pill_eq(self) -> None:
        new_val = not self.pill_eq.active
        self.pill_eq.set_active(new_val)
        self._async_cmd(lambda: self.client.set_eq(new_val))

    def _on_click_pill_peq(self) -> None:
        new_val = not self.pill_peq.active
        self.pill_peq.set_active(new_val)
        self._async_cmd(lambda: self.client.set_peq(new_val))

    def _on_click_pill_peq_profile(self) -> None:
        cur_prof = self.current_state.peq_profile if self.current_state else 1
        new_prof = 2 if cur_prof == 1 else 1
        self.pill_peq_profile.set_label(f"PEQ P{new_prof}")
        if self.current_state:
            self.current_state.peq_profile = new_prof
        self._async_cmd(lambda: self.client.set_peq_profile(new_prof))

    def _open_dsp_dialog(self) -> None:
        DspDialog(
            parent=self,
            client=self.client,
            current_state=self.current_state,
            theme_mgr=self.theme_mgr,
            on_change=lambda: None,
        )

    def _get_fav_glyphs(self) -> tuple[str, str]:
        """Return (filled, outline) favorite icons based on user preference."""
        if settings.effective_fav_mark == "star":
            return "★", "☆"
        return "♥", "♡"

    def _update_fav_ui(self) -> None:
        """Updates sidebar button label and Now Playing favorite button."""
        mark = settings.effective_fav_mark
        fav_btn = self.nav_buttons.get("favorites")
        if fav_btn:
            icon = "⭐" if mark == "star" else "♥"
            fav_btn.config(text=f"{icon}  Favourites")

        filled, outline = self._get_fav_glyphs()
        if hasattr(self, "btn_fav"):
            is_fav = bool(self.current_state and self.current_state.is_favorite)
            self.btn_fav.config(
                text=filled if is_fav else outline,
                fg=self.theme_mgr.accent_color if is_fav else THEME["text_secondary"],
            )

    def _on_accent_updated(self) -> None:
        """Re-tint components when the accent color changes."""
        color = self.theme_mgr.accent_color
        if hasattr(self, "btn_play"):
            self.btn_play.config(bg=color)
        if hasattr(self, "lbl_desktop_badge"):
            self.lbl_desktop_badge.config(fg=color)
        if hasattr(self, "lbl_audio_badge"):
            self.lbl_audio_badge.config(fg=color)
        if hasattr(self, "dsp_tune_btn"):
            self.dsp_tune_btn.config(fg=color)
        if hasattr(self, "btn_fav") and self.current_state and self.current_state.is_favorite:
            self.btn_fav.config(fg=color)

    def _open_settings_dialog(self) -> None:
        """Opens the full Settings modal window matching Android app."""
        SettingsDialog(
            parent=self,
            client=self.client,
            theme_mgr=self.theme_mgr,
            artwork_cache=self.artwork_cache,
            library_db=self.library_db,
            on_accent_changed=self._on_accent_updated,
            on_fav_mark_changed=self._update_fav_ui,
            on_resync_db=self._sync_database_manual,
            on_open_dsp=self._open_dsp_dialog,
            on_open_connect=self._show_connect_dialog,
        )

    # -------------------------------------------------------------------------
    # State Polling & UI Synchronization
    # -------------------------------------------------------------------------

    def _start_state_poller(self) -> None:
        def poller():
            while self.polling_active:
                state = self.client.get_state()
                if state:
                    self.ui_queue.put(lambda s=state: self._apply_player_state(s))
                else:
                    self.ui_queue.put(self._on_player_disconnected)
                time.sleep(1.0)

        t = threading.Thread(target=poller, daemon=True)
        t.start()

    def _process_ui_queue(self) -> None:
        while not self.ui_queue.empty():
            try:
                task = self.ui_queue.get_nowait()
                task()
            except Exception:
                pass
        self.after(50, self._process_ui_queue)

    def _on_player_disconnected(self) -> None:
        self.lbl_status_dot.config(fg=THEME["status_red"])

    def _apply_player_state(self, state: PlayerState) -> None:
        self.current_state = state
        self.lbl_status_dot.config(fg=THEME["status_green"])

        # Battery badge
        if state.battery >= 0:
            bolt = "⚡ " if state.charging else "🔋 "
            self.lbl_battery.config(text=f"{bolt}{state.battery}%")
        else:
            self.lbl_battery.config(text="")

        # Remember device choices reported by player
        if state.fav_star is not None:
            settings.device_fav_mark = "star" if state.fav_star else "heart"
        if state.fav_mark:
            settings.device_fav_mark = state.fav_mark.lower()

        # Dynamic accent color: if user preference is "device", follow player
        if state.accent_color:
            settings.device_accent = state.accent_color
            if settings.accent_preference == "device":
                self.theme_mgr.set_accent(state.accent_color)

        self.btn_play.config(bg=self.theme_mgr.accent_color)
        self.lbl_desktop_badge.config(fg=self.theme_mgr.accent_color)
        self.lbl_audio_badge.config(fg=self.theme_mgr.accent_color)
        if hasattr(self, "dsp_tune_btn"):
            self.dsp_tune_btn.config(fg=self.theme_mgr.accent_color)

        # Controls & icons
        self.btn_play.config(text="⏸" if state.is_playing else "▶")
        mode_icons = {"single": "🔂", "repeat": "🔁", "shuffle": "🔀", "normal": "➡"}
        self.btn_mode.config(text=mode_icons.get(state.playback_mode, "🔁"))

        filled, outline = self._get_fav_glyphs()
        self.btn_fav.config(
            text=filled if state.is_favorite else outline,
            fg=self.theme_mgr.accent_color if state.is_favorite else THEME["text_secondary"],
        )

        # Metadata
        self.lbl_track_title.config(text=state.title or (os.path.basename(state.path) if state.path else "No Track Playing"))
        self.lbl_track_artist.config(text=state.artist or state.album or "TempoTec V1")
        
        badge_text = state.audio_format_badge
        if badge_text:
            self.lbl_audio_badge.config(text=badge_text)
            self.lbl_audio_badge.pack(pady=(4, 10))
        else:
            self.lbl_audio_badge.pack_forget()

        # Seekbar
        if not self.is_seeking:
            self.seek_slider.config(to=max(1, state.duration))
            self.seek_var.set(state.position)
            self.lbl_time_elapsed.config(text=format_duration(state.position) or "0:00")
            self.lbl_time_total.config(text=format_duration(state.duration) or "0:00")

        # Queue counter
        if state.queue_total > 0:
            self.lbl_queue_pos.config(text=f"Queue {state.queue_index + 1}/{state.queue_total}")
        else:
            self.lbl_queue_pos.config(text="")

        # Volume
        self.vol_var.set(state.volume)
        self.lbl_vol_val.config(text=f"{state.volume}%")
        self.btn_mute.config(text="🔇" if state.muted else "🔊")

        # DSP Quick-Pill Status
        self.pill_mseb.set_active(state.mseb)
        self.pill_eq.set_active(state.eq)
        self.pill_peq.set_active(state.peq)
        self.pill_peq_profile.set_label(f"PEQ P{state.peq_profile}")
        self.pill_peq_profile.set_active(state.peq)

        # Artwork fetch
        if state.path and state.path != self.current_art_path:
            self.current_art_path = state.path
            self._refresh_artwork()

    def _refresh_artwork(self, force: bool = False) -> None:
        """Download and draw rounded cover art on the 220x220 canvas."""
        path = self.current_art_path
        if not path:
            self._draw_art_placeholder()
            return

        def worker():
            photo = self.artwork_cache.get_artwork(self.client, path, size=(220, 220), force=force)
            self.ui_queue.put(lambda: self._draw_art_photo(photo))

        threading.Thread(target=worker, daemon=True).start()

    def _draw_art_photo(self, photo: Optional[Any]) -> None:
        self.art_canvas.delete("all")
        if photo:
            self._current_photo_ref = photo
            self.art_canvas.create_image(110, 110, image=photo)
        else:
            self._draw_art_placeholder()

    def _draw_art_placeholder(self) -> None:
        self.art_canvas.delete("all")
        # Sleek dark vinyl disc placeholder
        self.art_canvas.create_rectangle(0, 0, 220, 220, fill=THEME["bg_card"], width=0)
        self.art_canvas.create_oval(18, 18, 202, 202, fill="#121318", outline="#262933", width=2)
        self.art_canvas.create_oval(45, 45, 175, 175, outline="#20222A", width=1)
        self.art_canvas.create_oval(70, 70, 150, 150, fill="#171920", outline="#2C303D", width=2)
        self.art_canvas.create_oval(100, 100, 120, 120, fill="#0C0D10", outline="#3B3F50", width=2)
        self.art_canvas.create_text(110, 110, text="🎵", font=("Segoe UI", 13), fill=THEME["text_secondary"])

    # -------------------------------------------------------------------------
    # Devices Dialog
    # -------------------------------------------------------------------------

    def _show_connect_dialog(self) -> None:
        dlg = tk.Toplevel(self)
        dlg.title("Devices on Network - SonixLink")
        dlg.geometry("440x380")
        dlg.configure(bg=THEME["bg_base"])
        dlg.transient(self)
        dlg.grab_set()

        self._active_connect_dialog = dlg

        hdr = tk.Label(
            dlg,
            text="Discovered Players",
            bg=THEME["bg_base"],
            fg=THEME["text_primary"],
            font=("Segoe UI", 12, "bold"),
            anchor="w",
        )
        hdr.pack(fill=tk.X, padx=16, pady=(14, 4))

        sub = tk.Label(
            dlg,
            text="Scanning local network for SonixPlayer / TempoTec V1...",
            bg=THEME["bg_base"],
            fg=THEME["text_secondary"],
            font=("Segoe UI", 8),
            anchor="w",
        )
        sub.pack(fill=tk.X, padx=16, pady=(0, 10))

        list_frame = tk.Frame(dlg, bg=THEME["bg_base"], padx=16)
        list_frame.pack(fill=tk.BOTH, expand=True)

        listbox = tk.Listbox(
            list_frame,
            bg=THEME["bg_card"],
            fg=THEME["text_primary"],
            selectbackground=self.theme_mgr.accent_color,
            selectforeground="#FFFFFF",
            font=("Segoe UI", 9),
            relief=tk.FLAT,
            highlightbackground=THEME["border"],
            highlightthickness=1,
        )
        listbox.pack(fill=tk.BOTH, expand=True)
        self._active_listbox = listbox

        beacons = list(self.discovered_devices.values())
        for b in beacons:
            listbox.insert(tk.END, f"{b.name} ({b.ip}:{b.port})")

        # Pre-fill active player option
        if not beacons and self.client.host:
            listbox.insert(tk.END, f"TempoTec V1 ({self.client.host}:{self.client.port})")

        # Manual IP input
        manual_frame = tk.Frame(dlg, bg=THEME["bg_base"], padx=16, pady=10)
        manual_frame.pack(fill=tk.X)

        tk.Label(manual_frame, text="Host IP:", bg=THEME["bg_base"], fg=THEME["text_secondary"]).pack(side=tk.LEFT)
        entry_ip = tk.Entry(manual_frame, bg=THEME["bg_field"], fg=THEME["text_primary"], width=16)
        entry_ip.insert(0, self.client.host or "192.168.0.17")
        entry_ip.pack(side=tk.LEFT, padx=(6, 12))

        tk.Label(manual_frame, text="Port:", bg=THEME["bg_base"], fg=THEME["text_secondary"]).pack(side=tk.LEFT)
        entry_port = tk.Entry(manual_frame, bg=THEME["bg_field"], fg=THEME["text_primary"], width=6)
        entry_port.insert(0, str(self.client.port or 7800))
        entry_port.pack(side=tk.LEFT, padx=(6, 0))

        def on_connect():
            sel = listbox.curselection()
            if sel and sel[0] < len(beacons):
                chosen = beacons[sel[0]]
                hip, hport = chosen.ip, chosen.port
                hname = chosen.name
            else:
                hip = entry_ip.get().strip() or "192.168.0.17"
                try:
                    hport = int(entry_port.get().strip())
                except ValueError:
                    hport = 7800
                hname = "TempoTec V1"

            self.client.set_target(hip, hport)
            settings.last_host = hip
            settings.last_port = hport
            self.lbl_dev_name.config(text=hname)
            self.lbl_dev_host.config(text=f"{hip}:{hport}")
            self._active_connect_dialog = None
            self._active_listbox = None
            dlg.destroy()
            threading.Thread(target=self._check_and_auto_sync, daemon=True).start()

        btn_box = tk.Frame(dlg, bg=THEME["bg_base"], padx=16, pady=10)
        btn_box.pack(fill=tk.X)

        btn_rescan = tk.Button(
            btn_box,
            text="Rescan ↻",
            command=lambda: threading.Thread(target=self.discovery.start, daemon=True).start(),
            bg=THEME["bg_field"],
            fg=THEME["text_primary"],
            relief=tk.FLAT,
            font=("Segoe UI", 9),
            padx=10,
            pady=4,
            cursor="hand2",
        )
        btn_rescan.pack(side=tk.LEFT)

        btn_go = tk.Button(
            btn_box,
            text="Connect",
            command=on_connect,
            bg=self.theme_mgr.accent_color,
            fg="#FFFFFF",
            relief=tk.FLAT,
            font=("Segoe UI", 9, "bold"),
            padx=16,
            pady=4,
            cursor="hand2",
        )
        btn_go.pack(side=tk.RIGHT)

        def on_close():
            self._active_connect_dialog = None
            self._active_listbox = None
            dlg.destroy()

        dlg.protocol("WM_DELETE_WINDOW", on_close)

    # -------------------------------------------------------------------------
    # Helper
    # -------------------------------------------------------------------------

    def _async_cmd(self, func: Callable[[], Any]) -> None:
        threading.Thread(target=func, daemon=True).start()

    def destroy(self) -> None:
        self.polling_active = False
        self.discovery.stop()
        super().destroy()
