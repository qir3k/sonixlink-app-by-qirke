# SonixLink App for TempoTec Variations V1 (by qirke)

An Android and Windows 64-bit desktop application that drives **Sonix Player** on the **TempoTec V1** and browses its audio library.

This fork targets the **TempoTec V1 only**. It is built on
[Jepl4r/sonixlink-app](https://github.com/Jepl4r/sonixlink-app) and adds a
translated interface, Genres and Years tabs, customisable tabs, accent and
favourite-icon choices, streaming-service browsing, hardware DSP controls (MSEB, Graphic EQ, PEQ, and profile switching), a native **Windows 64-bit Desktop version**, release updates, and
improved artwork and volume-step handling. It lives at
[qir3k/sonixlink-app-by-qirke](https://github.com/qir3k/sonixlink-app-by-qirke).

---

## Windows 64-bit Desktop App

In addition to the Android client, this repository includes a dedicated **Windows 64-bit Desktop Application** located in [\desktop/\](desktop/README.md):
- **Widescreen 3-Panel Layout**: Tailored for 16:9 / 16:10 desktop monitors (Navigation & Discovery on the left, sortable Library & File Browser in the center, Now Playing with 260x260 artwork and hardware DSP on the right).
- **DSP Controls**: Live pills for MSEB, Graphic EQ, PEQ, and PEQ Profile 1/2 switching with dynamic accent lighting.
- **Standalone Launchers**: Run via \SonixLink.exe\ (native 64-bit GUI launcher without console windows) or \SonixLink.bat\.
- See [\desktop/README.md\](desktop/README.md) for full details and usage.

---

## Why this fork

Highlights added or expanded in this fork:

| | This fork |
| --- | --- |
| **12 languages** | English, Čeština, Deutsch, Español, Français, Italiano, 日本語, Nederlands, Polski, Português (Brasil), Русский and Українська, with a language picker in the menu. The app follows the phone's language by default and can be switched per app, without changing the phone's language. |
| **Genres tab** | A tab of its own: every genre in the index, how many tracks it holds, its tracks, and circle-play (random order, in sequence, or a random track to the queue). |
| **Years tab** | Every release year in the index, oldest first, with how many tracks each holds, its tracks, and the same circle-play choices as Genres. |
| **Streaming tab** | Browse YouTube Music, Tidal, Qobuz, Podcasts and Internet radio from the phone: service menus, search, albums, playlists, shows, episodes and stations. Selecting playable content asks the connected player to start playback; audio is not streamed to the phone. The player supplies service accounts, keys, stream handling, and its own menu order. |
| **Tab order and visibility** | Reorder tabs by dragging the grip or with the up/down buttons, switch a tab off with its checkbox, and reset to the default order. At least one tab always stays on. |
| **Accent color picker** | *From player* (default) keeps following the player's accent, or pick one of 14 fixed colors — blue, red, orange, yellow, lime, green, cyan, indigo, purple, pink, brown, slate, amber or white. The choice applies to every open screen at once. |
| **Heart or star, detected** | The favourite button and the Favourites tab icon follow the mark the player uses: a heart or a star, read from the player itself. Choose *Auto (from player)*, *Heart* or *Star* by hand. |
| **Artwork everywhere** | Library covers come from the player's thumbnails; a missing album thumbnail can be fetched from embedded track art (`/api/art`). Now-playing and streaming artwork are also loaded from the player, with service icons as fallbacks. Artwork is cached for lists, playback, and the blurred background. |
| **Fast second launch** | The library index, the list of thumbnail keys and every thumbnail already fetched are kept on the phone and reopened locally. A later launch asks the player only for its fingerprints and downloads nothing when the library has not changed. |
| **Live artwork over Wi-Fi and Bluetooth** | Artwork is fetched while the lists scroll — 32 keys a request over Wi-Fi, 8 with a pause over Bluetooth — and a cover the player is still rendering is retried until it is ready. A cover skipped past is never fetched. |
| **Smooth scrolling** | When thumbnails arrive only the visible rows are rebound, on a short debounce, instead of the whole list — no ghosting tails while flinging on a 120 Hz screen. |
| **App updates** | The three-dot menu checks GitHub Releases, compares version tags, and can download the signed APK with Android's download notification. Android still requires the user to approve installation. Releases include a SHA-256 checksum; the `dev` test workflow builds a debug APK. |

Everything upstream does is still here: discovery over mDNS and the UDP beacon,
the search page, playback controls and modes, selection mode, the full queue
page with history, the media notification, Bluetooth, folder browsing, the
player's own sort order and favourite handling, and the crash log.

---

## Functionalities

- **Finds the player on its own.** Two independent routes, so only one of them has to work: DNS-SD (`_sonixlink._tcp`) and a plain UDP beacon on port 7801. The last player that answered is reconnected to without asking.
- **Browses the whole library** in ten tabs — Tracks, Albums, Artists, Album artists, Genres, Years, Playlists, Favourites, Folders (the card's folders, as the player's own file browser lists them) and Streaming. The tab order is up to you, and tabs you do not use can be hidden.
- **Browses the streaming services** the player already has — YouTube Music, Tidal, Qobuz, Podcasts and Internet radio — with a page for each: home rows, search, albums and playlists, show episodes, radio stations and radio browsing by country, genre or language. Tapping a track, episode or station starts it on the player, and the player queues the rest of the page.
- **Search** with a page of its own split into tracks, albums and artists.
- **Controls**: previous, play/pause, next, the five playback modes, the favourite mark and the queue.
- **Circle-play** on the lists the player has it on: play in random order, in sequence, or add a random track to the queue — including a whole genre.
- **Selection mode** as on the player: a long press on a track, a record, an artist or a playlist, then add the chosen tracks to the queue, the favourites or a playlist (or take them out of the favourites or the playlist they are in).
- **The queue page of the player**: Now playing at the top and what is to play below, fetched page by page as you scroll; the history above, a heading per day, held back when pulled down from Now playing. Select rows to take them out, drag them by the grip to reorder them, tap an entry of the history to play it next or long-press it to drop a copy anywhere in what is to play.
- **A media notification** with the track playing on the player, its artwork, previous/play-pause/next and the position bar, also on the lock screen, so the app does not need to be opened to change track.
- **Over Wi-Fi or Bluetooth.** A player paired with the phone can be reached without a shared Wi-Fi network.
- **Now-playing screen** with the cover sent by the player and a blurred version of the same image behind it.
- **The phone's volume keys can change the device volume**, with the app open, from the lock screen, and system-wide even when the phone screen is off or from a connected smartwatch. The app uses the player's volume step (1–5%) and keeps volume controls responsive across screen states.
- **The accent color follows the player** by default; change it there and every screen of the app changes with it, at once. Prefer a fixed color? Pick one in the menu and the app stops following the player.

### The settings menu

| Setting | Default | Choices |
| --- | --- | --- |
| Language | System default (the phone's language) | 11 languages, or back to System default |
| Tab order | Tracks, Albums, Artists, Album artists, Genres, Years, Playlists, Favourites, Folders, Streaming | Drag or arrows to reorder; uncheck to hide; Reset |
| Accent color | From player (device default) | 14 fixed colors |
| Favourite icon | Auto (from player) | Auto, Heart, Star |
| Volume step | From player | 1-5 % per press, sent to the player |
| Check for app updates | — | Check GitHub for a newer release, download with Android's progress notification, then approve installation in Android |
| Back up app settings | — | Save settings in Downloads/SonixLink |
| Change device | — | Disconnect and choose another player |
| Refresh the index | — | Fetch the current library index and covers again |

App choices such as language, accent, favourite mark, volume step and tab
visibility/order are kept between launches. A normal app update signed with the
same key also keeps Android's private app data automatically.

The app also mirrors its SharedPreferences to a uniquely named
`sonixlink-settings-*.json` file in `Download/SonixLink` on Android 10 and later.
**Back up app settings** in the three-dot menu writes a fresh copy and reports
success or failure; a newly installed build with no settings restores the
latest copy at startup. This supports reinstalling when Android requires
removing a build signed with a different key. No broad storage permission is
needed: the app
uses Android's Downloads provider. Android 7–9 does not support this automatic
external backup, so a reinstall there loses app settings. Only preferences are
copied; the library index and covers are downloaded from the player again.

---

## How it works

The app downloads the player's **own SQLite index** once (`/api/db`) and queries
it locally. The covers in the lists are the player's **own thumbnails**, fetched a
screenful at a time as the lists scroll (`/api/thumbs`) and kept on the phone:
the player's thumbnail file can be a hundred megabytes, and only the list of
which thumbnails exist (`/api/thumbkeys`, eight bytes each) is read at sync.
For an album whose thumbnail the player has never drawn, the app asks for the
track's embedded artwork directly (`/api/art`) and stores it in the same local
thumbnail store, so the album gets a cover without waiting for the player to be
browsed first.

What is transmitted after is the state and the commands, which are two lines of
JSON, plus the full artwork for the now-playing screen.

### The protocol

Plain HTTP/1.1 with JSON answers on port **7800**. No binary framing, no session;
anything that looks odd opens in a browser and shows itself.

The player shows SonixLink's icon in its status bar for as long as requests keep
arriving (20 s after the last one). The app keeps a foreground service running
while it is connected — the same one that owns the media notification — which
reads the state every 2 s while the player plays and every 5 s in pause, and
says `/api/bye` when it disconnects.

| Request | Answer |
| --- | --- |
| `GET /api/info` | name, model, firmware, serial, track count, whether it is scanning, size, timestamp and fingerprint (`db_hash`, `covers_hash`) of the index and of the thumbnails, accent |
| `GET /api/state` | what is playing: state, mode, volume, position, duration, title/artist/album/path, format, battery, queue position, DSP states (`mseb`, `eq`, `peq`, `peq_profile`) |
| `GET /api/db` | the SQLite index, as a file (`503` while the player is scanning) |
| `GET /api/thumbkeys` | every thumbnail key the player holds, 8 bytes each (the 64-bit key, little-endian) |
| `GET /api/thumbs?keys=<k>,<k>…` | up to 64 thumbnails: for each, the 16-digit key, width and height (u16 LE), length (u32 LE), RGB565 pixels; 0×0 is "no artwork", unknown keys are left out |
| `GET /api/covers` | the whole thumbnail store, as a file (kept for older apps) |
| `GET /api/queue[?from=<n>]` | up to 200 paths of the queue: around the playing track, or from row `n`; with `count`, `position`, `revision` (changes whenever the queue does) and `pending` (the player has not fetched that stretch yet: ask again) |
| `GET /api/browse[?path=<dir>]` | one folder of the card, folders first: `{path, root, parent, entries:[{name, path, dir}]}`; confined to the card |
| `GET /api/ping` | `ok` — the heartbeat, for a client that needs nothing else |
| `GET /api/bye` | the app is leaving: the player drops the icon at once |
| `GET /api/favourites` | the favourites as they stand right now |
| `GET /api/history[?limit=<n>]` | the latest `n` (200, at most 1000) entries of what the player has played, oldest first: `{revision, items:[{id, path, title, artist, played_at}]}`; `history_revision` in the state changes when one is added |
| `GET /api/art?path=[&max=<px>]` | one track's artwork, whole and undecoded; with `max` fitted into that many pixels and sent as a JPEG of a few tens of KB. A fitted cover is made on a thread of its own: until it is ready the answer is `202` and the app asks again a moment later (the player also makes the next track's cover ahead, at the size last asked for) |
| `POST /api/command?do=…` | `play`, `pause`, `toggle`, `next`, `prev`, `seek&value=<seconds>`, `volume&value=<0-100>`, `mode&value=<normal\|repeat_all\|repeat_one\|shuffle\|shuffle_repeat>`, `mseb[&value=0\|1]`, `eq[&value=0\|1]`, `peq[&value=0\|1]`, `peq_profile[&value=1\|2\|next]`, `play_path&path=<path>[&list=<all\|album\|artist\|album_artist\|genre\|year\|favourites\|playlist\|queue\|folder>&value=<name>]`, `favourite&path=<path>[&value=0\|1]`, `queue_index&value=<n>`, `scan`, `play_all&list=<…\|year\|albums>&value=<name>&how=<sequence\|shuffle\|random>` (the circle-play menu) |
| `POST /api/command?do=<queue_next\|favourites_add\|favourites_remove\|playlist_add\|playlist_remove>[&value=<playlist>]` | a selection of tracks, as the player's selection mode: the paths in the body, one a line |
| `POST /api/command?do=queue_remove&rev=<r>` | the queue entries in the body (indices, one a line) taken out |
| `POST /api/command?do=queue_move&from=<i>&to=<j>&rev=<r>` | one entry moved, `to` being where it ends up |
| `POST /api/command?do=history_play&id=<id>` | an entry of the history put in right after the track playing, and played |
| `POST /api/command?do=history_insert&id=<id>&at=<slot>&rev=<r>` | an entry of the history put in the queue at `slot` |
| `GET /api/stream/services` | the five streaming services, with whether each is configured and signed in |
| `GET /api/stream/list?service=&kind=&id=&q=&offset=` | one page of a service: `{title, count, more, error, rows:[{kind, id, title, subtitle, album, art, url, mime, duration, track_number, playable}]}`. `kind=menu` returns the player's own menu for that service instead of network content |
| `POST /api/stream/play?service=&index=` | plays row `index` of that service's last page and queues the rest of its tracks, episodes or stations |

The queue commands change only what comes after the track playing, and `rev`
is the queue revision they were worked out against: the player drops a
command meant for a queue that has changed since.

`play_path` carries the list the track was touched in, so the queue becomes that
list.

### Bluetooth

The same requests travel over an RFCOMM link. With SonixLink and Bluetooth both
on, the player registers an SDP service with the UUID
`8d6e3a52-4c1f-4b7e-9a2d-5f0c7e1b3a90` on RFCOMM channel **22**; the app
connects to it by UUID, falls back to channel 22 directly when the phone's SDP
lookup does not find it, and sends one
request after another on the same link (`Connection: keep-alive`, every answer
with a `Content-Length`). The player closes a link idle for 30 s and the app
opens it again on the next request.

The phone has to be **paired** with the player first, from the phone's
Bluetooth settings. Then **Connect over Bluetooth** on the first screen lists
the paired devices. Over Bluetooth the index downloads more slowly than over
Wi-Fi (a few hundred KB/s).

Over Bluetooth every request goes one after another on the same link, so the
app keeps what crosses it small: covers come scaled by the player (640 px,
about 30-60 KB instead of the megabytes a cover can be in the file), the
thumbnails eight at a time with a pause between requests, the cover of a track
already skipped past is never fetched, the state read and the commands go ahead
of any cover or thumbnail still waiting, and the notification's service does
not read the state again when a screen of the app has just read it.

Headphones can stay connected to the player at the same time: the phone is
treated as the remote control, never as the audio device, and connecting
headphones does not push it off. If the phone starts sending its own sound to
the player after pairing, switch off **Media audio** (and **Calls**) for the
player in the phone's Bluetooth settings: SonixLink needs neither.

### Streaming

The streaming pages are a remote view of the player's own clients, not a
second streaming implementation: the phone asks the player for a page
(`/api/stream/list`), the player calls its YouTube Music, Tidal, Qobuz,
Podcasts or radio module, and the answer is returned as one row shape. Tapping
a playable row sends the service and row index (`/api/stream/play`); the player
resolves and plays the audio and owns the resulting queue. The app does not
download or decode the stream itself. This means YouTube Music playback fixes
to stream resolution or playback belong in the Sonix Player firmware; SonixLink
provides the browsing and remote-control interface. Accounts, API keys and
sign-in state stay on the player: a service without them is shown as *Not set
up on the player* or *Sign in on the player*.

The rows inside each service are the player's own menu, not a list this app
invented. `kind=menu` reads the same settings the player's pages write —
Tidal's `home_order`/`home_hidden`, YouTube Music's `home_order`/`home_hidden`,
radio's `hub_order`/`hub_hidden` — so Tidal, YouTube Music and radio show the
entries, order and on/off state the DAP has, and a change made there appears on
the phone the next time a service's home page is opened. Qobuz (favourite
tracks, favourite albums, my playlists, new releases) and Podcasts (my podcasts,
trending) are the same fixed menus the player has. Each service has its own
icon, both in the chooser and on its rows. Streaming rows also load the artwork
URL supplied by the player, with the service icon kept as a fallback.

Tidal, YouTube Music, Qobuz and Podcasts have a **Downloaded** entry in the app.
It lists kept files in the player's `Music/Tidal`, `Music/YouTube Music`,
`Music/Qobuz` and `Podcast` folders that are present in the phone's library
index. If a newly kept item is missing,
use **Refresh the index** in the app menu after the player has indexed it; tapping
a listed item starts playback on the player.

### The order of the lists

The lists read in the player's own order: `/api/info` and `/api/state` carry
`sort` — which lists the player has reversed (Z-A) or put by date added, and
whether an artist's tracks are grouped by record — and the app orders its
copies the same way, redrawing a tab when that changes on the player. An album
opens by disc and track number, and two records with the same name are two
albums, told apart by the `album_key` the player writes beside every track.
A track tapped in the app gets the queue the player's own list would give.

### What is downloaded, and when

The index is kept on the phone (`library.db`) and downloaded again only when its
fingerprint (`db_hash` in `/api/info`) changes; the list of thumbnail keys
(`thumbkeys.txt`) likewise with `covers_hash`, and thumbnails already fetched
(`thumbnails.db`) are never fetched again. Album covers resolved from embedded
art are stored the same way. The fingerprint covers what the app reads — the
library tables and the playlists, the thumbnail keys — and not what the player
writes on its own all the time (where playback was left, the saved queue), so a
connection with nothing new on the player downloads nothing. The copy on the
phone is tied to the player's serial, so the same player over Wi-Fi and over
Bluetooth shares it.

The second launch therefore opens the local index and thumbnail store and shows
the library without downloading it again; only a changed library, a different
player, or **Refresh the index** in the menu starts a fresh download.

The now-playing cover and the notification keep the last four covers in memory,
and the player is asked for one cover at a time so only the track that ends up
playing crosses the link.

### Discovery

- **mDNS / DNS-SD** — the player answers for `_sonixlink._tcp.local` with
  PTR + SRV + TXT + A in one packet and re-announces every 30 s. This is what
  Android's `NsdManager` uses.
- **A plain UDP beacon** on port **7801**, broadcast every 2 s, reading
  `SONIXLINK1 <ip> <port> <name>`.

### If the app closes itself

The trace of the last crash goes to `last-crash.txt` in the app's own data, and
is shown on the next launch with a **Copy** button. That is the thing to send —
not "it closed itself".

---

## Requirements

- Android 7.0 (API 24) or newer.
- A **TempoTec V1** running firmware with **Sonix Player** and SonixLink, with
  **Wireless --> SonixLink** switched on, on the same Wi-Fi network or paired
  with the phone over Bluetooth.
- Other players are not supported by this fork.
- Streaming needs the matching service configured on the player: an API key or
  relay for YouTube Music, a Tidal or Qobuz account, Podcast Index keys for
  podcasts, and a network connection for radio.

## Building

Nothing beyond Android Studio (Koala or newer) is needed:

1. `File > Open`, and choose this folder.
2. Studio fetches Gradle 8.7 and the dependencies on the first sync.
3. `Run` on the phone, or `Build > Build APK(s)` for an APK to install by hand.

From the command line, with JDK 17 and the Android SDK installed:

```sh
./gradlew assembleDebug
```

The release workflow publishes a tagged APK and its SHA-256 checksum on every
push to `main`; the test workflow builds the debug APK on `dev` and on pull
requests. The app can check the latest published release directly from the
connection screen (before connecting) as well as from the three-dot menu. Android
does not allow an ordinary app to silently replace itself: after the download,
tap the notification and approve the install in Android's package installer. If
Android asks, allow SonixLink to install unknown apps.

### Signing releases

Every update must be signed with the same private key as the installed app.
Create one release keystore and keep a secure offline backup; if it is lost,
Android will not accept future updates signed by a replacement key:

```powershell
keytool -genkeypair -v -storetype JKS -keystore sonixlink-release.jks -alias sonixlink -keyalg RSA -keysize 4096 -validity 10000
$env:SONIX_KEYSTORE_BASE64 = [Convert]::ToBase64String([IO.File]::ReadAllBytes("$PWD\sonixlink-release.jks"))
$env:SONIX_KEYSTORE_BASE64 | Set-Clipboard
```

In the GitHub repository, open **Settings > Secrets and variables > Actions**
and add these repository secrets:

| Secret | Value |
| --- | --- |
| `SONIX_KEYSTORE_BASE64` | The copied base64 keystore |
| `SONIX_KEYSTORE_PASSWORD` | The keystore password entered during creation |
| `SONIX_KEY_ALIAS` | `sonixlink` |
| `SONIX_KEY_PASSWORD` | The key password entered during creation |

Do not commit the keystore or put its passwords in Gradle files. The release
workflow fails rather than publishing an APK with a temporary debug key if
these secrets are missing. Release version codes increase automatically.

This signing change cannot make an already installed debug-signed APK accept a
new release-signed APK. Back up preferences as described above, uninstall that
old build once, and install the first APK signed with the release keystore.
Subsequent releases made with the same keystore can install as updates without
uninstalling.

The Streaming tab needs the `/api/stream/*` routes in the
[Sonix Player firmware](https://github.com/qir3k/sonix-player-by-qirke). Without
them the tab opens with no services; the rest of the app is unaffected.

## Credits

SonixLink was written by [Jepl4r](https://github.com/Jepl4r) and is maintained
here as a TempoTec V1 fork by
[qir3k](https://github.com/qir3k/sonixlink-app-by-qirke).
