"""
SonixLink Windows Desktop - Audio & DSP Dialog
Modal / floating dialog for configuring MSEB, Graphic EQ, and Parametric EQ profiles.
"""

from __future__ import annotations

import os
import threading
import tkinter as tk
from tkinter import ttk
from typing import Callable, Optional

from sonixlink_client import PlayerState, SonixLinkClient
from ui_theme import THEME, ThemeManager


class DspDialog(tk.Toplevel):
    """
    Dialog for tuning audio DSP processing on SonixPlayer.
    Provides live hardware toggles for:
      - MSEB (Mage Sound 8-Ball)
      - Graphic Equalizer
      - Parametric Equalizer (PEQ)
      - PEQ Profile 1 / 2 selection
    """

    def __init__(
        self,
        parent: tk.Tk | tk.Toplevel,
        client: SonixLinkClient,
        current_state: Optional[PlayerState],
        theme_mgr: ThemeManager,
        on_change: Optional[Callable[[], None]] = None,
    ):
        super().__init__(parent)
        self.client = client
        self.theme_mgr = theme_mgr
        self.on_change = on_change

        self.title("Audio & DSP Settings - SonixLink")
        self.geometry("520x540")
        self.minsize(460, 480)
        self.configure(bg=THEME["bg_base"])
        self.transient(parent)
        self.grab_set()

        desktop_dir = os.path.dirname(os.path.abspath(__file__))
        ico_path = os.path.join(desktop_dir, "icon.ico")
        if os.path.exists(ico_path):
            try:
                self.iconbitmap(ico_path)
            except Exception:
                pass

        # State variables
        self.var_mseb = tk.BooleanVar(value=bool(current_state and current_state.mseb))
        self.var_eq = tk.BooleanVar(value=bool(current_state and current_state.eq))
        self.var_peq = tk.BooleanVar(value=bool(current_state and current_state.peq))
        current_prof = current_state.peq_profile if current_state else 1
        self.var_peq_profile = tk.IntVar(value=current_prof if current_prof in (1, 2) else 1)

        self._build_ui()
        self._center_window(parent)
        self.focus_set()

    def _center_window(self, parent: tk.Tk | tk.Toplevel) -> None:
        self.update_idletasks()
        try:
            pw = parent.winfo_width()
            ph = parent.winfo_height()
            px = parent.winfo_rootx()
            py = parent.winfo_rooty()
            w = self.winfo_width()
            h = self.winfo_height()
            x = px + max(0, (pw - w) // 2)
            y = py + max(0, (ph - h) // 2)
            self.geometry(f"+{x}+{y}")
        except Exception:
            pass

    def _build_ui(self) -> None:
        accent = self.theme_mgr.accent_color

        # Header Frame
        hdr = tk.Frame(self, bg=THEME["bg_card"], padx=20, pady=16)
        hdr.pack(fill=tk.X, side=tk.TOP)

        title_lbl = tk.Label(
            hdr,
            text="Audio & DSP Effects",
            bg=THEME["bg_card"],
            fg=THEME["text_primary"],
            font=("Segoe UI", 14, "bold"),
            anchor="w",
        )
        title_lbl.pack(fill=tk.X)

        sub_lbl = tk.Label(
            hdr,
            text="Control hardware audio processing engines directly on SonixPlayer.",
            bg=THEME["bg_card"],
            fg=THEME["text_secondary"],
            font=("Segoe UI", 9),
            anchor="w",
        )
        sub_lbl.pack(fill=tk.X, pady=(2, 0))

        # Main Body
        body = tk.Frame(self, bg=THEME["bg_base"], padx=20, pady=16)
        body.pack(fill=tk.BOTH, expand=True)

        # Card 1: MSEB
        self._build_card(
            body,
            title="MSEB (Mage Sound 8-Ball)",
            desc="Sound tuning algorithms based on parametric EQ and sound field adjustments for warmth, transient response, and texture.",
            var=self.var_mseb,
            on_toggle=self._on_toggle_mseb,
        )

        # Card 2: Graphic EQ
        self._build_card(
            body,
            title="Graphic Equalizer",
            desc="Multi-band frequency equalizer for fine-tuning tonal balance across bass, midrange, and treble spectrum.",
            var=self.var_eq,
            on_toggle=self._on_toggle_eq,
        )

        # Card 3: Parametric EQ (PEQ) + Profile selector
        peq_card = tk.Frame(body, bg=THEME["bg_card"], padx=14, pady=12, highlightbackground=THEME["border"], highlightthickness=1)
        peq_card.pack(fill=tk.X, pady=(0, 12))

        peq_top = tk.Frame(peq_card, bg=THEME["bg_card"])
        peq_top.pack(fill=tk.X)

        peq_info = tk.Frame(peq_top, bg=THEME["bg_card"])
        peq_info.pack(side=tk.LEFT, fill=tk.BOTH, expand=True)

        peq_title = tk.Label(
            peq_info,
            text="Parametric EQ (PEQ)",
            bg=THEME["bg_card"],
            fg=THEME["text_primary"],
            font=("Segoe UI", 11, "bold"),
            anchor="w",
        )
        peq_title.pack(fill=tk.X)

        peq_desc = tk.Label(
            peq_info,
            text="Precise parametric filters with customizable gain, center frequencies, and Q-factor bandwidths.",
            bg=THEME["bg_card"],
            fg=THEME["text_secondary"],
            font=("Segoe UI", 8),
            wraplength=340,
            justify=tk.LEFT,
            anchor="w",
        )
        peq_desc.pack(fill=tk.X, pady=(2, 0))

        self.peq_check = tk.Checkbutton(
            peq_top,
            variable=self.var_peq,
            command=self._on_toggle_peq,
            text="",
            bg=THEME["bg_card"],
            activebackground=THEME["bg_card"],
            selectcolor=THEME["bg_field"],
            cursor="hand2",
        )
        self.peq_check.pack(side=tk.RIGHT, padx=4)

        # Profile selection row inside PEQ card
        prof_frame = tk.Frame(peq_card, bg=THEME["bg_card"], pady=8)
        prof_frame.pack(fill=tk.X, pady=(6, 0))

        prof_label = tk.Label(
            prof_frame,
            text="Active Profile:",
            bg=THEME["bg_card"],
            fg=THEME["text_secondary"],
            font=("Segoe UI", 9, "bold"),
        )
        prof_label.pack(side=tk.LEFT, padx=(0, 12))

        self.rb_prof1 = tk.Radiobutton(
            prof_frame,
            text="Profile 1",
            variable=self.var_peq_profile,
            value=1,
            command=self._on_select_profile,
            bg=THEME["bg_card"],
            fg=THEME["text_primary"],
            activebackground=THEME["bg_card"],
            selectcolor=THEME["bg_field"],
            font=("Segoe UI", 9),
            cursor="hand2",
        )
        self.rb_prof1.pack(side=tk.LEFT, padx=(0, 12))

        self.rb_prof2 = tk.Radiobutton(
            prof_frame,
            text="Profile 2",
            variable=self.var_peq_profile,
            value=2,
            command=self._on_select_profile,
            bg=THEME["bg_card"],
            fg=THEME["text_primary"],
            activebackground=THEME["bg_card"],
            selectcolor=THEME["bg_field"],
            font=("Segoe UI", 9),
            cursor="hand2",
        )
        self.rb_prof2.pack(side=tk.LEFT)

        # Bottom Bar
        bottom = tk.Frame(self, bg=THEME["bg_base"], padx=20, pady=12)
        bottom.pack(fill=tk.X, side=tk.BOTTOM)

        tip_lbl = tk.Label(
            bottom,
            text="Changes take effect immediately on playback hardware.",
            bg=THEME["bg_base"],
            fg=THEME["text_muted"],
            font=("Segoe UI", 8),
            anchor="w",
        )
        tip_lbl.pack(side=tk.LEFT)

        close_btn = tk.Button(
            bottom,
            text="Done",
            command=self.destroy,
            bg=accent,
            fg="#FFFFFF",
            activebackground=accent,
            activeforeground="#FFFFFF",
            font=("Segoe UI", 9, "bold"),
            relief=tk.FLAT,
            padx=18,
            pady=6,
            cursor="hand2",
        )
        close_btn.pack(side=tk.RIGHT)

    def _build_card(
        self,
        parent: tk.Widget,
        title: str,
        desc: str,
        var: tk.BooleanVar,
        on_toggle: Callable[[], None],
    ) -> None:
        card = tk.Frame(parent, bg=THEME["bg_card"], padx=14, pady=12, highlightbackground=THEME["border"], highlightthickness=1)
        card.pack(fill=tk.X, pady=(0, 12))

        info = tk.Frame(card, bg=THEME["bg_card"])
        info.pack(side=tk.LEFT, fill=tk.BOTH, expand=True)

        tlbl = tk.Label(
            info,
            text=title,
            bg=THEME["bg_card"],
            fg=THEME["text_primary"],
            font=("Segoe UI", 11, "bold"),
            anchor="w",
        )
        tlbl.pack(fill=tk.X)

        dlbl = tk.Label(
            info,
            text=desc,
            bg=THEME["bg_card"],
            fg=THEME["text_secondary"],
            font=("Segoe UI", 8),
            wraplength=340,
            justify=tk.LEFT,
            anchor="w",
        )
        dlbl.pack(fill=tk.X, pady=(2, 0))

        chk = tk.Checkbutton(
            card,
            variable=var,
            command=on_toggle,
            text="",
            bg=THEME["bg_card"],
            activebackground=THEME["bg_card"],
            selectcolor=THEME["bg_field"],
            cursor="hand2",
        )
        chk.pack(side=tk.RIGHT, padx=4)

    def update_from_state(self, state: PlayerState) -> None:
        self.var_mseb.set(bool(state.mseb))
        self.var_eq.set(bool(state.eq))
        self.var_peq.set(bool(state.peq))
        if state.peq_profile in (1, 2):
            self.var_peq_profile.set(state.peq_profile)

    def _on_toggle_mseb(self) -> None:
        val = self.var_mseb.get()
        threading.Thread(target=lambda: self.client.set_mseb(val), daemon=True).start()
        if self.on_change:
            self.on_change()

    def _on_toggle_eq(self) -> None:
        val = self.var_eq.get()
        threading.Thread(target=lambda: self.client.set_eq(val), daemon=True).start()
        if self.on_change:
            self.on_change()

    def _on_toggle_peq(self) -> None:
        val = self.var_peq.get()
        threading.Thread(target=lambda: self.client.set_peq(val), daemon=True).start()
        if self.on_change:
            self.on_change()

    def _on_select_profile(self) -> None:
        prof = self.var_peq_profile.get()
        threading.Thread(target=lambda: self.client.set_peq_profile(prof), daemon=True).start()
        if self.on_change:
            self.on_change()
