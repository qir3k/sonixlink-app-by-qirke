"""
Theme, colors, fonts, and custom widgets for SonixLink Desktop.
Matches the Android app's modern dark aesthetic and dynamically follows the player's accent color.
"""

from __future__ import annotations

import tkinter as tk
from tkinter import ttk
from typing import Callable, Optional

# Premium Modern Dark Theme Palette
THEME = {
    "bg_base": "#0C0D10",
    "bg_panel": "#131418",
    "bg_card": "#1A1C23",
    "bg_card_hover": "#222530",
    "bg_field": "#20232D",
    "border": "#242733",
    "border_light": "#323747",
    "text_primary": "#FFFFFF",
    "text_secondary": "#A0A6B5",
    "text_muted": "#606675",
    "accent_default": "#FF8C00",  # Matches TempoTec V1 theme
    "status_green": "#2ECC71",
    "status_red": "#E74C3C",
    "row_even": "#101116",
    "row_odd": "#14161C",
}


def format_duration(seconds: int | float) -> str:
    """Format total seconds into mm:ss or hh:mm:ss string."""
    try:
        s = int(seconds)
    except Exception:
        return ""
    if s <= 0:
        return ""
    m, sec = divmod(s, 60)
    h, m = divmod(m, 60)
    if h > 0:
        return f"{h}:{m:02d}:{sec:02d}"
    return f"{m}:{sec:02d}"


class ThemeManager:
    """Manages dynamic accent color updates and ttk styling."""

    def __init__(self, root: tk.Tk):
        self.root = root
        self.accent_color: str = THEME["accent_default"]
        self.style = ttk.Style(root)
        self.listeners: list[Callable[[str], None]] = []
        self.apply_ttk_styles()

    def set_accent(self, accent_hex: str) -> None:
        if not accent_hex or not accent_hex.startswith("#") or len(accent_hex) != 7:
            return
        if self.accent_color.upper() == accent_hex.upper():
            return
        self.accent_color = accent_hex
        self.apply_ttk_styles()
        for cb in self.listeners:
            try:
                cb(self.accent_color)
            except Exception:
                pass

    def add_accent_listener(self, cb: Callable[[str], None]) -> None:
        self.listeners.append(cb)

    def apply_ttk_styles(self) -> None:
        try:
            self.style.theme_use("clam")
        except Exception:
            pass

        # Global default for ttk
        self.style.configure(
            ".",
            background=THEME["bg_panel"],
            foreground=THEME["text_primary"],
            font=("Segoe UI", 9),
        )

        # Frames
        self.style.configure("TFrame", background=THEME["bg_panel"])
        self.style.configure("Base.TFrame", background=THEME["bg_base"])
        self.style.configure("Card.TFrame", background=THEME["bg_card"])

        # Treeview (Modern library table)
        self.style.configure(
            "Treeview",
            background=THEME["bg_base"],
            foreground=THEME["text_primary"],
            fieldbackground=THEME["bg_base"],
            bordercolor=THEME["border"],
            borderwidth=0,
            rowheight=30,
            font=("Segoe UI", 9),
        )
        self.style.map(
            "Treeview",
            background=[("selected", self.accent_color)],
            foreground=[("selected", "#FFFFFF")],
        )
        self.style.configure(
            "Treeview.Heading",
            background=THEME["bg_panel"],
            foreground=THEME["text_muted"],
            font=("Segoe UI", 8, "bold"),
            relief="flat",
            borderwidth=0,
            padding=(8, 6),
        )
        self.style.map(
            "Treeview.Heading",
            background=[("active", THEME["bg_card"])],
            foreground=[("active", THEME["text_primary"])],
        )

        # Scrollbars (Smooth dark style)
        self.style.configure(
            "Vertical.TScrollbar",
            background=THEME["bg_card"],
            troughcolor=THEME["bg_base"],
            bordercolor=THEME["bg_base"],
            arrowcolor=THEME["text_muted"],
            relief="flat",
            borderwidth=0,
        )

        # Horizontal Sliders (Seek & Volume)
        self.style.configure(
            "Horizontal.TScale",
            background=THEME["bg_panel"],
            troughcolor=THEME["bg_card"],
            sliderthickness=12,
            sliderrelief="flat",
            borderwidth=0,
        )


class DspPillWidget(tk.Canvas):
    """
    A custom-drawn rounded pill button matching the hardware player's illuminated pills:
    Active: Filled with dynamic accent color + bold white text
    Inactive: Matte dark card + subtle border + secondary text
    """

    def __init__(
        self,
        parent: tk.Widget,
        label: str = "",
        text: str = "",
        active: bool = False,
        theme_mgr: Optional[ThemeManager] = None,
        accent: Optional[str] = None,
        on_click: Optional[Callable[[], None]] = None,
        width: int = 56,
        height: int = 24,
    ):
        bg = parent["bg"] if "bg" in parent.keys() else THEME["bg_card"]
        super().__init__(parent, width=width, height=height, bg=bg, highlightthickness=0)

        self.label = label or text or "DSP"
        self.active = active
        self.theme_mgr = theme_mgr
        self._custom_accent = accent
        self.on_click = on_click
        self.width = width
        self.height = height
        self._hover = False

        if self.theme_mgr:
            self.theme_mgr.add_accent_listener(lambda _: self.redraw())

        self.bind("<Button-1>", self._handle_click)
        self.bind("<Enter>", self._handle_enter)
        self.bind("<Leave>", self._handle_leave)
        self.redraw()

    @property
    def current_accent(self) -> str:
        if self._custom_accent:
            return self._custom_accent
        if self.theme_mgr:
            return self.theme_mgr.accent_color
        return THEME["accent_default"]

    def set_active(self, active: bool) -> None:
        if self.active != active:
            self.active = active
            self.redraw()

    def set_label(self, label: str) -> None:
        if self.label != label:
            self.label = label
            self.redraw()

    def set_text(self, text: str) -> None:
        self.set_label(text)

    def set_accent(self, accent: str) -> None:
        self._custom_accent = accent
        self.redraw()

    def _handle_click(self, _: tk.Event) -> None:
        if self.on_click:
            self.on_click()

    def _handle_enter(self, _: tk.Event) -> None:
        self._hover = True
        self.redraw()

    def _handle_leave(self, _: tk.Event) -> None:
        self._hover = False
        self.redraw()

    def redraw(self) -> None:
        self.delete("all")
        r = self.height / 2
        w, h = self.width, self.height
        accent = self.current_accent

        if self.active:
            bg_color = accent
            text_color = "#FFFFFF"
            border_color = accent
        else:
            bg_color = THEME["bg_card_hover"] if self._hover else THEME["bg_card"]
            text_color = THEME["text_primary"] if self._hover else THEME["text_secondary"]
            border_color = THEME["border_light"] if self._hover else THEME["border"]

        # Draw smooth rounded pill shape
        self.create_arc(1, 1, h, h, start=90, extent=180, fill=bg_color, outline=border_color)
        self.create_arc(w - h - 1, 1, w - 1, h, start=270, extent=180, fill=bg_color, outline=border_color)
        self.create_rectangle(r, 1, w - r, h, fill=bg_color, outline=bg_color)
        # Top and bottom border lines
        self.create_line(r, 1, w - r, 1, fill=border_color)
        self.create_line(r, h, w - r, h, fill=border_color)

        # Centered crisp typography
        self.create_text(w / 2, h / 2, text=self.label, fill=text_color, font=("Segoe UI", 8, "bold"))
