"""
SonixLink Windows Desktop Application - Main Entry Point
Launches the 64-bit desktop client for SonixPlayer.
"""

from __future__ import annotations

import argparse
import ctypes
import os
import sys
import traceback

# Ensure current directory is on sys.path
SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
if SCRIPT_DIR not in sys.path:
    sys.path.insert(0, SCRIPT_DIR)


def enable_windows_dpi_awareness() -> None:
    """Enable high-DPI awareness on Windows for sharp fonts and widgets."""
    if sys.platform != "win32":
        return
    try:
        # Per-monitor DPI awareness (Windows 8.1+)
        ctypes.windll.shcore.SetProcessDpiAwareness(1)
    except Exception:
        try:
            # System DPI awareness (Windows Vista+)
            ctypes.windll.user32.SetProcessDPIAware()
        except Exception:
            pass


def show_fatal_error(err: Exception) -> None:
    """Log crash traceback and display Windows message box."""
    tb = traceback.format_exc()
    repo_root = os.path.dirname(SCRIPT_DIR)
    log_path = os.path.join(repo_root, "sonixlink_crash.log")
    try:
        with open(log_path, "w", encoding="utf-8") as f:
            f.write(tb)
    except Exception:
        pass

    if sys.platform == "win32":
        try:
            msg = f"SonixLink encountered an error:\n\n{err}\n\nDetails saved to: {log_path}"
            ctypes.windll.user32.MessageBoxW(0, msg, "SonixLink Error", 0x10)
        except Exception:
            pass


def main() -> None:
    try:
        parser = argparse.ArgumentParser(description="SonixLink Windows 64-bit Desktop Remote App")
        parser.add_argument("--host", default=None, help="Target SonixPlayer IP address (default: saved last host)")
        parser.add_argument("--port", type=int, default=None, help="Target SonixPlayer HTTP port (default: saved last port)")
        args = parser.parse_args()

        enable_windows_dpi_awareness()

        from main_window import MainWindow

        app = MainWindow(host=args.host, port=args.port)
        app.mainloop()
    except Exception as e:
        show_fatal_error(e)
        sys.exit(1)


if __name__ == "__main__":
    main()
