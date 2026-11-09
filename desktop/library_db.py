"""
SQLite database manager for SonixLink local library queries.
Directly reads the index database downloaded from SonixPlayer (/api/db).
"""

from __future__ import annotations

import os
import sqlite3
import tempfile
from typing import Any, Dict, List, Optional


def _get_default_db_path() -> str:
    # 1. Prefer database in the repository root / app directory
    repo_dir = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    local_db = os.path.join(repo_dir, "library.db")
    try:
        test_f = os.path.join(repo_dir, ".write_test")
        with open(test_f, "w") as f:
            f.write("1")
        os.remove(test_f)
        return local_db
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
            return os.path.join(c, "library.db")
        except Exception:
            continue
    return os.path.join(os.getcwd(), "library.db")


class LibraryDb:
    def __init__(self, db_path: Optional[str] = None):
        self.db_path = db_path or _get_default_db_path()
        self._conn: Optional[sqlite3.Connection] = None

    def reload(self) -> bool:
        self.close()
        return self.connect()

    def is_available(self) -> bool:
        return bool(self.db_path and os.path.exists(self.db_path) and os.path.getsize(self.db_path) > 0)

    def connect(self) -> bool:
        if not self.is_available():
            return False
        try:
            self._conn = sqlite3.connect(f"file:{self.db_path}?mode=ro", uri=True, check_same_thread=False)
            self._conn.row_factory = sqlite3.Row
            return True
        except Exception as e:
            print(f"LibraryDb connect error: {e}")
            self._conn = None
            return False

    def close(self) -> None:
        if self._conn:
            try:
                self._conn.close()
            except Exception:
                pass
            self._conn = None

    def _query(self, sql: str, params: tuple = ()) -> List[sqlite3.Row]:
        if not self._conn:
            if not self.connect():
                return []
        try:
            cur = self._conn.cursor()
            cur.execute(sql, params)
            return cur.fetchall()
        except Exception as e:
            print(f"SQL Error: {e} | Query: {sql}")
            return []

    def get_track_count(self) -> int:
        rows = self._query("SELECT COUNT(*) AS c FROM MEDIA_TABLE WHERE path <> ''")
        return rows[0]["c"] if rows else 0

    def get_tracks(self, limit: int = 10000) -> List[Dict[str, Any]]:
        rows = self._query(
            """SELECT name AS title, artist, album, path, ctime, mtime, size, sortkey,
                      ck_id AS track_num,
                      (CASE WHEN end_time > begin_time THEN (end_time - begin_time) / 1000 ELSE 0 END) AS duration
               FROM MEDIA_TABLE
               WHERE path <> ''
               ORDER BY sortkey, name
               LIMIT ?""",
            (limit,)
        )
        return [dict(r) for r in rows]

    def get_all_tracks(self, limit: int = 15000) -> List[Dict[str, Any]]:
        return self.get_tracks(limit=limit)

    def get_albums(self) -> List[Dict[str, Any]]:
        rows = self._query(
            """SELECT album AS name, artist,
                      (SELECT COUNT(*) FROM MEDIA_TABLE m WHERE m.album = t.album AND m.path <> '') AS track_count
               FROM ALBUM_TABLE t
               ORDER BY sortkey, album"""
        )
        if not rows:
            rows = self._query(
                """SELECT DISTINCT album AS name, artist,
                          COUNT(*) AS track_count
                   FROM MEDIA_TABLE
                   WHERE album <> '' AND path <> ''
                   GROUP BY album
                   ORDER BY album"""
            )
        return [dict(r) for r in rows]

    def get_album_tracks(self, album_name: str) -> List[Dict[str, Any]]:
        rows = self._query(
            """SELECT name AS title, artist, album, path, ctime, mtime, size,
                      ck_id AS track_num,
                      (CASE WHEN end_time > begin_time THEN (end_time - begin_time) / 1000 ELSE 0 END) AS duration
               FROM MEDIA_TABLE
               WHERE album = ? AND path <> ''
               ORDER BY ck_id, sortkey, name""",
            (album_name,)
        )
        return [dict(r) for r in rows]

    def get_artists(self) -> List[Dict[str, Any]]:
        rows = self._query(
            """SELECT artist AS name,
                      (SELECT COUNT(*) FROM MEDIA_TABLE m WHERE m.artist = t.artist AND m.path <> '') AS track_count
               FROM ARTIST_TABLE t
               ORDER BY sortkey, artist"""
        )
        if not rows:
            rows = self._query(
                """SELECT DISTINCT artist AS name,
                          COUNT(*) AS track_count
                   FROM MEDIA_TABLE
                   WHERE artist <> '' AND path <> ''
                   GROUP BY artist
                   ORDER BY artist"""
            )
        return [dict(r) for r in rows]

    def get_artist_tracks(self, artist_name: str) -> List[Dict[str, Any]]:
        rows = self._query(
            """SELECT name AS title, artist, album, path, ctime, mtime, size,
                      ck_id AS track_num,
                      (CASE WHEN end_time > begin_time THEN (end_time - begin_time) / 1000 ELSE 0 END) AS duration
               FROM MEDIA_TABLE
               WHERE artist = ? AND path <> ''
               ORDER BY album, ck_id, sortkey, name""",
            (artist_name,)
        )
        return [dict(r) for r in rows]

    def get_genres(self) -> List[Dict[str, Any]]:
        rows = self._query(
            """SELECT genre AS name,
                      (SELECT COUNT(*) FROM MEDIA_TABLE m WHERE m.genre = t.genre AND m.path <> '') AS track_count
               FROM GENRE_TABLE t
               ORDER BY sortkey, genre"""
        )
        if not rows:
            rows = self._query(
                """SELECT DISTINCT genre AS name,
                          COUNT(*) AS track_count
                   FROM MEDIA_TABLE
                   WHERE genre <> '' AND path <> ''
                   GROUP BY genre
                   ORDER BY genre"""
            )
        return [dict(r) for r in rows]

    def get_genre_tracks(self, genre_name: str) -> List[Dict[str, Any]]:
        rows = self._query(
            """SELECT name AS title, artist, album, path, ctime, mtime, size,
                      ck_id AS track_num,
                      (CASE WHEN end_time > begin_time THEN (end_time - begin_time) / 1000 ELSE 0 END) AS duration
               FROM MEDIA_TABLE
               WHERE genre = ? AND path <> ''
               ORDER BY sortkey, name""",
            (genre_name,)
        )
        return [dict(r) for r in rows]

    def get_years(self) -> List[Dict[str, Any]]:
        rows = self._query(
            """SELECT CAST(year AS TEXT) AS name,
                      (SELECT COUNT(*) FROM MEDIA_TABLE m WHERE m.year = t.year AND m.path <> '') AS track_count
               FROM YEAR_TABLE t
               ORDER BY sortkey DESC, year DESC"""
        )
        if not rows:
            rows = self._query(
                """SELECT DISTINCT CAST(year AS TEXT) AS name,
                          COUNT(*) AS track_count
                   FROM MEDIA_TABLE
                   WHERE year > 0 AND path <> ''
                   GROUP BY year
                   ORDER BY year DESC"""
            )
        return [dict(r) for r in rows]

    def get_year_tracks(self, year_str: str) -> List[Dict[str, Any]]:
        rows = self._query(
            """SELECT name AS title, artist, album, path, ctime, mtime, size,
                      ck_id AS track_num,
                      (CASE WHEN end_time > begin_time THEN (end_time - begin_time) / 1000 ELSE 0 END) AS duration
               FROM MEDIA_TABLE
               WHERE CAST(year AS TEXT) = ? AND path <> ''
               ORDER BY sortkey, name""",
            (year_str,)
        )
        return [dict(r) for r in rows]

    def get_favourites(self) -> List[Dict[str, Any]]:
        rows = self._query(
            """SELECT f.name AS title, f.artist, f.path,
                      m.album, m.ck_id AS track_num,
                      (CASE WHEN m.end_time > m.begin_time THEN (m.end_time - m.begin_time) / 1000 ELSE 0 END) AS duration
               FROM FAVORITE_TABLE f
               LEFT JOIN MEDIA_TABLE m ON m.path = f.path
               WHERE f.path <> ''
               ORDER BY f.rowid DESC"""
        )
        return [dict(r) for r in rows]

    def get_favorites(self) -> List[Dict[str, Any]]:
        return self.get_favourites()

    def get_playlists(self) -> List[Dict[str, Any]]:
        tables = self._query(
            "SELECT name FROM sqlite_master WHERE type='table' AND name LIKE 'M3U\\_%' ESCAPE '\\' ORDER BY name"
        )
        playlists = []
        for t in tables:
            tname = t["name"]
            clean_name = tname[4:] if tname.startswith("M3U_") else tname
            count_rows = self._query(f'SELECT COUNT(*) AS c FROM "{tname}"')
            c = count_rows[0]["c"] if count_rows else 0
            playlists.append({"id": tname, "name": clean_name, "track_count": c})
        return playlists

    def get_playlist_tracks(self, table_name: str) -> List[Dict[str, Any]]:
        rows = self._query(
            f"""SELECT p.title, p.artist, p.path,
                       m.album, m.ck_id AS track_num,
                       (CASE WHEN m.end_time > m.begin_time THEN (m.end_time - m.begin_time) / 1000 ELSE 0 END) AS duration
                FROM "{table_name}" p
                LEFT JOIN MEDIA_TABLE m ON m.path = p.path
                WHERE p.path <> ''
                ORDER BY p.rowid"""
        )
        return [dict(r) for r in rows]

    def search(self, query_str: str, limit: int = 250) -> List[Dict[str, Any]]:
        term = f"%{query_str.strip()}%"
        rows = self._query(
            """SELECT name AS title, artist, album, path,
                      ck_id AS track_num,
                      (CASE WHEN end_time > begin_time THEN (end_time - begin_time) / 1000 ELSE 0 END) AS duration
               FROM MEDIA_TABLE
               WHERE (name LIKE ? OR artist LIKE ? OR album LIKE ?) AND path <> ''
               ORDER BY sortkey, name
               LIMIT ?""",
            (term, term, term, limit)
        )
        return [dict(r) for r in rows]

    def search_tracks(self, query_str: str, limit: int = 250) -> List[Dict[str, Any]]:
        return self.search(query_str, limit=limit)


LibraryDatabase = LibraryDb
