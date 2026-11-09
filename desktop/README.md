# SonixLink for Windows (64-bit)

A dedicated, native-feeling desktop controller for **SonixPlayer** (TempoTec V1 / custom audiophile DAP firmware) designed specifically for Windows 64-bit widescreen displays (16:9 and 16:10).

---

## Features

### 1. Widescreen 3-Panel Layout
- **Left Panel (Navigation & Sources):**
  - Live connection card with player hostname, port, and battery status.
  - One-click UDP network auto-discovery and manual connection dialog.
  - "Sync DB" button to download and query the SQLite database locally (`/api/db`).
  - Search box querying songs, artists, and albums in real time.
  - Categorized browsing: Tracks, Albums, Artists, Genres, Years, Favourites, Playlists, SD Card File Browser, Play Queue, and Streaming services.
  - Dedicated **Audio & DSP Settings** dialog button.
- **Center Panel (Interactive Library Browser):**
  - Smooth sortable table views with custom column configurations.
  - Double-click to play songs, expand albums, artists, genres, or drill down SD card folders.
  - Right-click context menu: **Play Now**, **Play Next**, **Add to Queue**, **Toggle Favourite**, **Copy File Path**.
- **Right Panel (Now Playing & Hardware DSP):**
  - 260x260 high-resolution album artwork canvas with disk caching and placeholder art.
  - Title, artist, album labels and queue position (`Queue 12/85`).
  - Interactive seek slider with elapsed and total timestamps (`mm:ss`).
  - Mode toggle (Normal, Repeat All, Repeat Single, Shuffle), Prev, Next, Play/Pause circle accent button, and Favourite star.
  - **DSP Engine Quick Pills**:
    - `[ MSEB ]` - Mage Sound 8-Ball toggle.
    - `[ EQ ]` - Graphic Equalizer toggle.
    - `[ PEQ ]` - Parametric Equalizer toggle.
    - `[ PEQ P1 / P2 ]` - Switch active PEQ profile between Profile 1 and Profile 2.
    - Dynamic hardware accent color glow when active.
  - Volume slider (0–100%) and mute toggle.

### 2. Audio & DSP Dialog
- Accessible via the **Audio & DSP Settings** button or Now Playing sidebar.
- Dedicated hardware controls for:
  - **MSEB (Mage Sound 8-Ball)**: Audio warmth, transient response, and bass field adjustments.
  - **Graphic Equalizer**: Multi-band tonal balance across the audio spectrum.
  - **Parametric Equalizer (PEQ)**: Custom parametric filters, gain, center frequencies, and Q-factor bandwidths.
  - **PEQ Profile Selection**: Radio switch between **Profile 1** and **Profile 2**.
- All adjustments take effect immediately on hardware via non-blocking asynchronous commands.

### 3. UDP Auto-Discovery & Zero-Config Networking
- Continuously listens for UDP beacon broadcasts on port `7801` (`SONIXLINK1 <ip> <port> <name>`).
- Seamlessly auto-connects to discovered SonixPlayer devices on the local Wi-Fi / LAN.
- Supports manual IP and port override when needed.

---

## How to Run

### Method 1: Standalone Native Launcher (Recommended)
Double-click `SonixLink.exe` in the repository root.
- 64-bit native Windows GUI executable.
- Launches directly without displaying a command prompt console window.

### Method 2: Batch Script
Double-click `SonixLink.bat` in the repository root.

### Method 3: Command Line
```powershell
# From the repository root:
python desktop/app.py

# Or specify custom host / port:
python desktop/app.py --host 192.168.1.105 --port 7800
```

---

## Requirements
- Windows 10 / 11 64-bit
- Python 3.10+ with `Pillow` and standard library (`tkinter`, `sqlite3`, `urllib`, `socket`)
- Network access on port 7800 (HTTP API) and 7801 (UDP Discovery)
