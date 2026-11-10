use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Clone, Default)]
pub struct TrackItem {
    pub track_num: u32,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration: u32,
    pub path: String,
}

#[derive(Debug, Clone, Default)]
pub struct SimpleItem {
    pub name: String,
    pub track_count: u32,
}

type SqliteOpen = unsafe extern "system" fn(filename: *const c_char, ppDb: *mut *mut c_void) -> c_int;
type SqliteClose = unsafe extern "system" fn(pDb: *mut c_void) -> c_int;
type SqlitePrepareV2 = unsafe extern "system" fn(
    pDb: *mut c_void,
    zSql: *const c_char,
    nByte: c_int,
    ppStmt: *mut *mut c_void,
    pzTail: *mut *const c_char,
) -> c_int;
type SqliteStep = unsafe extern "system" fn(pStmt: *mut c_void) -> c_int;
type SqliteFinalize = unsafe extern "system" fn(pStmt: *mut c_void) -> c_int;
type SqliteColumnText = unsafe extern "system" fn(pStmt: *mut c_void, iCol: c_int) -> *const c_char;
type SqliteColumnInt = unsafe extern "system" fn(pStmt: *mut c_void, iCol: c_int) -> c_int;

const SQLITE_ROW: c_int = 100;
const SQLITE_DONE: c_int = 101;
const SQLITE_OK: c_int = 0;

struct SqliteLib {
    _lib: libloading::Library,
    open: SqliteOpen,
    close: SqliteClose,
    prepare_v2: SqlitePrepareV2,
    step: SqliteStep,
    finalize: SqliteFinalize,
    column_text: SqliteColumnText,
    column_int: SqliteColumnInt,
}

pub struct LibraryDb {
    sqlite: Option<Arc<SqliteLib>>,
    pub db_path: PathBuf,
}

impl LibraryDb {
    pub fn new() -> Self {
        let sqlite = Self::load_sqlite();
        let db_path = Self::find_db_path();
        Self { sqlite, db_path }
    }

    fn load_sqlite() -> Option<Arc<SqliteLib>> {
        let lib = unsafe { libloading::Library::new("winsqlite3.dll") }.ok()?;
        unsafe {
            let open: SqliteOpen = *lib.get(b"sqlite3_open\0").ok()?;
            let close: SqliteClose = *lib.get(b"sqlite3_close\0").ok()?;
            let prepare_v2: SqlitePrepareV2 = *lib.get(b"sqlite3_prepare_v2\0").ok()?;
            let step: SqliteStep = *lib.get(b"sqlite3_step\0").ok()?;
            let finalize: SqliteFinalize = *lib.get(b"sqlite3_finalize\0").ok()?;
            let column_text: SqliteColumnText = *lib.get(b"sqlite3_column_text\0").ok()?;
            let column_int: SqliteColumnInt = *lib.get(b"sqlite3_column_int\0").ok()?;
            Some(Arc::new(SqliteLib {
                _lib: lib,
                open,
                close,
                prepare_v2,
                step,
                finalize,
                column_text,
                column_int,
            }))
        }
    }

    pub fn find_db_path() -> PathBuf {
        let mut candidates = Vec::new();

        if let Ok(exe) = std::env::current_exe() {
            if let Some(parent) = exe.parent() {
                candidates.push(parent.join("library.db"));
            }
        }
        candidates.push(PathBuf::from("library.db"));
        candidates.push(PathBuf::from("sonixlink_library.db"));

        if let Ok(local_app) = std::env::var("LOCALAPPDATA") {
            candidates.push(PathBuf::from(local_app).join("SonixLink").join("library.db"));
        }
        if let Ok(app_data) = std::env::var("APPDATA") {
            candidates.push(PathBuf::from(app_data).join("SonixLink").join("library.db"));
        }

        for c in &candidates {
            if c.exists() && c.metadata().map(|m| m.len() > 100).unwrap_or(false) {
                return c.clone();
            }
        }

        // Default write target if none exist yet
        if let Ok(local_app) = std::env::var("LOCALAPPDATA") {
            let p = PathBuf::from(local_app).join("SonixLink").join("library.db");
            let _ = std::fs::create_dir_all(p.parent().unwrap());
            return p;
        }

        PathBuf::from("library.db")
    }

    pub fn get_track_count(&self) -> usize {
        let lib = match &self.sqlite {
            Some(l) => l,
            None => return 0,
        };
        let c_path = match CString::new(self.db_path.to_string_lossy().as_bytes()) {
            Ok(p) => p,
            Err(_) => return 0,
        };
        let mut db: *mut c_void = std::ptr::null_mut();
        if unsafe { (lib.open)(c_path.as_ptr(), &mut db) } != SQLITE_OK {
            return 0;
        }

        let mut count = 0;
        let sql = CString::new("SELECT COUNT(*) FROM MEDIA_TABLE WHERE path <> ''").unwrap();
        let mut stmt: *mut c_void = std::ptr::null_mut();
        if unsafe { (lib.prepare_v2)(db, sql.as_ptr(), -1, &mut stmt, std::ptr::null_mut()) } == SQLITE_OK {
            if unsafe { (lib.step)(stmt) } == SQLITE_ROW {
                count = unsafe { (lib.column_int)(stmt, 0) as usize };
            }
            unsafe { (lib.finalize)(stmt) };
        }
        unsafe { (lib.close)(db) };
        count
    }

    pub fn get_all_tracks(&self) -> Vec<TrackItem> {
        let query = "SELECT name AS title, artist, album, path, \
                     (CASE WHEN end_time > begin_time THEN (end_time - begin_time) / 1000 ELSE 0 END) AS duration, \
                     ck_id AS track_num \
                     FROM MEDIA_TABLE WHERE path <> '' ORDER BY sortkey, name LIMIT 5000";
        self.query_tracks(query)
    }

    pub fn get_tracks_by_album(&self, album: &str) -> Vec<TrackItem> {
        let safe_album = album.replace('\'', "''");
        let query = format!(
            "SELECT name AS title, artist, album, path, \
             (CASE WHEN end_time > begin_time THEN (end_time - begin_time) / 1000 ELSE 0 END) AS duration, \
             ck_id AS track_num \
             FROM MEDIA_TABLE WHERE path <> '' AND album = '{}' ORDER BY ck_id, sortkey, name LIMIT 2000",
            safe_album
        );
        self.query_tracks(&query)
    }

    pub fn get_tracks_by_artist(&self, artist: &str) -> Vec<TrackItem> {
        let safe_artist = artist.replace('\'', "''");
        let query = format!(
            "SELECT name AS title, artist, album, path, \
             (CASE WHEN end_time > begin_time THEN (end_time - begin_time) / 1000 ELSE 0 END) AS duration, \
             ck_id AS track_num \
             FROM MEDIA_TABLE WHERE path <> '' AND artist = '{}' ORDER BY album, ck_id, sortkey, name LIMIT 2000",
            safe_artist
        );
        self.query_tracks(&query)
    }

    pub fn get_tracks_by_genre(&self, genre: &str) -> Vec<TrackItem> {
        let safe_genre = genre.replace('\'', "''");
        let query = format!(
            "SELECT name AS title, artist, album, path, \
             (CASE WHEN end_time > begin_time THEN (end_time - begin_time) / 1000 ELSE 0 END) AS duration, \
             ck_id AS track_num \
             FROM MEDIA_TABLE WHERE path <> '' AND genre = '{}' ORDER BY artist, album, name LIMIT 2000",
            safe_genre
        );
        self.query_tracks(&query)
    }

    pub fn get_tracks_by_year(&self, year: &str) -> Vec<TrackItem> {
        let safe_year = year.replace('\'', "''");
        let query = format!(
            "SELECT name AS title, artist, album, path, \
             (CASE WHEN end_time > begin_time THEN (end_time - begin_time) / 1000 ELSE 0 END) AS duration, \
             ck_id AS track_num \
             FROM MEDIA_TABLE WHERE path <> '' AND year = '{}' ORDER BY artist, album, name LIMIT 2000",
            safe_year
        );
        self.query_tracks(&query)
    }

    pub fn get_favourites(&self) -> Vec<TrackItem> {
        let query = "SELECT f.name AS title, f.artist, m.album, f.path, \
                     (CASE WHEN m.end_time > m.begin_time THEN (m.end_time - m.begin_time) / 1000 ELSE 0 END) AS duration, \
                     m.ck_id AS track_num \
                     FROM FAVORITE_TABLE f \
                     LEFT JOIN MEDIA_TABLE m ON m.path = f.path \
                     WHERE f.path <> '' ORDER BY f.rowid DESC";
        self.query_tracks(query)
    }

    fn query_tracks(&self, sql_str: &str) -> Vec<TrackItem> {
        let lib = match &self.sqlite {
            Some(l) => l,
            None => return Vec::new(),
        };
        let c_path = match CString::new(self.db_path.to_string_lossy().as_bytes()) {
            Ok(p) => p,
            Err(_) => return Vec::new(),
        };
        let mut db: *mut c_void = std::ptr::null_mut();
        if unsafe { (lib.open)(c_path.as_ptr(), &mut db) } != SQLITE_OK {
            return Vec::new();
        }

        let mut results = Vec::with_capacity(100);
        let sql = CString::new(sql_str).unwrap();
        let mut stmt: *mut c_void = std::ptr::null_mut();
        if unsafe { (lib.prepare_v2)(db, sql.as_ptr(), -1, &mut stmt, std::ptr::null_mut()) } == SQLITE_OK {
            while unsafe { (lib.step)(stmt) } == SQLITE_ROW {
                let title = self.extract_text(lib, stmt, 0);
                let artist = self.extract_text(lib, stmt, 1);
                let album = self.extract_text(lib, stmt, 2);
                let path = self.extract_text(lib, stmt, 3);
                let duration = unsafe { (lib.column_int)(stmt, 4) as u32 };
                let track_num = unsafe { (lib.column_int)(stmt, 5) as u32 };
                results.push(TrackItem {
                    track_num,
                    title,
                    artist,
                    album,
                    duration,
                    path,
                });
            }
            unsafe { (lib.finalize)(stmt) };
        }
        unsafe { (lib.close)(db) };
        results
    }

    pub fn get_albums(&self) -> Vec<SimpleItem> {
        self.query_simple("SELECT album AS name, COUNT(*) AS count FROM MEDIA_TABLE WHERE album <> '' GROUP BY album ORDER BY sortkey, album")
    }

    pub fn get_artists(&self) -> Vec<SimpleItem> {
        self.query_simple("SELECT artist AS name, COUNT(*) AS count FROM MEDIA_TABLE WHERE artist <> '' GROUP BY artist ORDER BY sortkey, artist")
    }

    pub fn get_genres(&self) -> Vec<SimpleItem> {
        self.query_simple("SELECT genre AS name, COUNT(*) AS count FROM MEDIA_TABLE WHERE genre <> '' GROUP BY genre ORDER BY sortkey, genre")
    }

    pub fn get_years(&self) -> Vec<SimpleItem> {
        self.query_simple("SELECT year AS name, COUNT(*) AS count FROM MEDIA_TABLE WHERE year <> '' GROUP BY year ORDER BY year DESC")
    }

    pub fn get_playlists(&self) -> Vec<SimpleItem> {
        let lib = match &self.sqlite {
            Some(l) => l,
            None => return Vec::new(),
        };
        let c_path = match CString::new(self.db_path.to_string_lossy().as_bytes()) {
            Ok(p) => p,
            Err(_) => return Vec::new(),
        };
        let mut db: *mut c_void = std::ptr::null_mut();
        if unsafe { (lib.open)(c_path.as_ptr(), &mut db) } != SQLITE_OK {
            return Vec::new();
        }

        let mut table_names = Vec::new();
        let sql = CString::new("SELECT name FROM sqlite_master WHERE type='table' AND name LIKE 'M3U_%' ORDER BY name").unwrap();
        let mut stmt: *mut c_void = std::ptr::null_mut();
        if unsafe { (lib.prepare_v2)(db, sql.as_ptr(), -1, &mut stmt, std::ptr::null_mut()) } == SQLITE_OK {
            while unsafe { (lib.step)(stmt) } == SQLITE_ROW {
                let name = self.extract_text(lib, stmt, 0);
                if !name.is_empty() {
                    table_names.push(name);
                }
            }
            unsafe { (lib.finalize)(stmt) };
        }

        let mut results = Vec::new();
        for table in table_names {
            let clean_name = table.strip_prefix("M3U_").unwrap_or(&table).to_string();
            let count_sql = format!("SELECT COUNT(*) FROM \"{}\" WHERE path <> ''", table.replace('"', "\"\""));
            let mut count = 0;
            if let Ok(c_sql) = CString::new(count_sql) {
                let mut count_stmt: *mut c_void = std::ptr::null_mut();
                if unsafe { (lib.prepare_v2)(db, c_sql.as_ptr(), -1, &mut count_stmt, std::ptr::null_mut()) } == SQLITE_OK {
                    if unsafe { (lib.step)(count_stmt) } == SQLITE_ROW {
                        count = unsafe { (lib.column_int)(count_stmt, 0) as u32 };
                    }
                    unsafe { (lib.finalize)(count_stmt) };
                }
            }
            results.push(SimpleItem {
                name: clean_name,
                track_count: count,
            });
        }

        unsafe { (lib.close)(db) };
        results
    }

    pub fn get_tracks_by_playlist(&self, playlist_name: &str) -> Vec<TrackItem> {
        let table = format!("M3U_{}", playlist_name.replace('"', "\"\""));
        let query = format!(
            "SELECT p.title, p.artist, IFNULL(m.album, '') AS album, p.path, \
             p.seconds AS duration, p.idx AS track_num \
             FROM \"{}\" p \
             LEFT JOIN MEDIA_TABLE m ON m.path = p.path \
             WHERE p.path <> '' ORDER BY p.idx LIMIT 2000",
            table
        );
        self.query_tracks(&query)
    }

    pub fn generate_smartmix(&self, seed_path: &str) -> Vec<TrackItem> {
        let lib = match &self.sqlite {
            Some(l) => l,
            None => return Vec::new(),
        };
        let c_path = match CString::new(self.db_path.to_string_lossy().as_bytes()) {
            Ok(p) => p,
            Err(_) => return Vec::new(),
        };
        let mut db: *mut c_void = std::ptr::null_mut();
        if unsafe { (lib.open)(c_path.as_ptr(), &mut db) } != SQLITE_OK {
            return Vec::new();
        }

        // 1. Fetch seed track metadata
        let mut seed_title = String::new();
        let mut seed_artist = String::new();
        let mut seed_album = String::new();
        let mut seed_genre = String::new();
        let mut seed_year: i32 = 0;
        let mut seed_duration: u32 = 0;
        let mut actual_seed_path = seed_path.to_string();

        let seed_sql = if !seed_path.is_empty() {
            format!(
                "SELECT name, artist, album, genre, year, \
                 (CASE WHEN end_time > begin_time THEN (end_time - begin_time)/1000 ELSE 0 END) AS duration \
                 FROM MEDIA_TABLE WHERE path = '{}' LIMIT 1",
                seed_path.replace('\'', "''")
            )
        } else {
            "SELECT name, artist, album, genre, year, \
             (CASE WHEN end_time > begin_time THEN (end_time - begin_time)/1000 ELSE 0 END) AS duration, \
             path \
             FROM MEDIA_TABLE WHERE path <> '' LIMIT 1".to_string()
        };

        if let Ok(c_sql) = CString::new(seed_sql) {
            let mut stmt: *mut c_void = std::ptr::null_mut();
            if unsafe { (lib.prepare_v2)(db, c_sql.as_ptr(), -1, &mut stmt, std::ptr::null_mut()) } == SQLITE_OK {
                if unsafe { (lib.step)(stmt) } == SQLITE_ROW {
                    seed_title = self.extract_text(lib, stmt, 0);
                    seed_artist = self.extract_text(lib, stmt, 1);
                    seed_album = self.extract_text(lib, stmt, 2);
                    seed_genre = self.extract_text(lib, stmt, 3);
                    seed_year = unsafe { (lib.column_int)(stmt, 4) };
                    seed_duration = unsafe { (lib.column_int)(stmt, 5) as u32 };
                    if seed_path.is_empty() {
                        actual_seed_path = self.extract_text(lib, stmt, 6);
                    }
                }
                unsafe { (lib.finalize)(stmt) };
            }
        }

        if seed_title.is_empty() {
            unsafe { (lib.close)(db) };
            return Vec::new();
        }

        // 2. Fetch candidate tracks
        struct RawCandidate {
            track: TrackItem,
            score: f32,
        }
        let mut candidates = Vec::with_capacity(500);

        let cand_sql = format!(
            "SELECT name, artist, album, genre, year, path, \
             (CASE WHEN end_time > begin_time THEN (end_time - begin_time)/1000 ELSE 0 END) AS duration \
             FROM MEDIA_TABLE WHERE path <> '{}' AND path <> '' LIMIT 2500",
            actual_seed_path.replace('\'', "''")
        );

        if let Ok(c_sql) = CString::new(cand_sql) {
            let mut stmt: *mut c_void = std::ptr::null_mut();
            if unsafe { (lib.prepare_v2)(db, c_sql.as_ptr(), -1, &mut stmt, std::ptr::null_mut()) } == SQLITE_OK {
                let mut pseudo_rand: u32 = 0x12345678;
                while unsafe { (lib.step)(stmt) } == SQLITE_ROW {
                    let title = self.extract_text(lib, stmt, 0);
                    let artist = self.extract_text(lib, stmt, 1);
                    let album = self.extract_text(lib, stmt, 2);
                    let genre = self.extract_text(lib, stmt, 3);
                    let year = unsafe { (lib.column_int)(stmt, 4) };
                    let path = self.extract_text(lib, stmt, 5);
                    let duration = unsafe { (lib.column_int)(stmt, 6) as u32 };

                    let mut score = 0.0f32;
                    if !seed_artist.is_empty() && artist.eq_ignore_ascii_case(&seed_artist) {
                        score += 50.0;
                    }
                    if !seed_genre.is_empty() && !genre.is_empty() && genre.eq_ignore_ascii_case(&seed_genre) {
                        score += 40.0;
                    }
                    if seed_year > 0 && year > 0 {
                        let diff = (year - seed_year).abs();
                        if diff <= 2 { score += 30.0; }
                        else if diff <= 5 { score += 20.0; }
                        else if diff <= 10 { score += 10.0; }
                    }

                    if score > 0.0 {
                        pseudo_rand = pseudo_rand.wrapping_mul(1103515245).wrapping_add(12345);
                        let jitter = ((pseudo_rand >> 16) % 15) as f32;
                        score += jitter;

                        candidates.push(RawCandidate {
                            track: TrackItem {
                                track_num: 0,
                                title,
                                artist,
                                album,
                                duration,
                                path,
                            },
                            score,
                        });
                    }
                }
                unsafe { (lib.finalize)(stmt) };
            }
        }

        unsafe { (lib.close)(db) };

        candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        // 3. Assemble 25-track mix with artist spacing
        let mut mix = Vec::with_capacity(25);
        mix.push(TrackItem {
            track_num: 1,
            title: seed_title,
            artist: seed_artist.clone(),
            album: seed_album,
            duration: seed_duration,
            path: actual_seed_path,
        });

        use std::collections::HashMap;
        let mut artist_counts = HashMap::new();
        if !seed_artist.is_empty() {
            artist_counts.insert(seed_artist.to_lowercase(), 1);
        }

        let mut pool: Vec<RawCandidate> = candidates.into_iter().take(200).collect();
        while !pool.is_empty() && mix.len() < 25 {
            let last_artist = mix.last().map(|t| t.artist.to_lowercase()).unwrap_or_default();
            let mut chosen_idx = None;

            for (idx, c) in pool.iter().enumerate() {
                let art_key = c.track.artist.to_lowercase();
                if art_key == last_artist && pool.len() > 4 {
                    continue;
                }
                let count = *artist_counts.get(&art_key).unwrap_or(&0);
                let max_allowed = if art_key == seed_artist.to_lowercase() { 3 } else { 2 };
                if count >= max_allowed && pool.len() > 8 {
                    continue;
                }
                chosen_idx = Some(idx);
                break;
            }

            let chosen = if let Some(idx) = chosen_idx {
                pool.remove(idx)
            } else {
                pool.remove(0)
            };

            let art_key = chosen.track.artist.to_lowercase();
            *artist_counts.entry(art_key).or_insert(0) += 1;

            let mut t = chosen.track;
            t.track_num = (mix.len() + 1) as u32;
            mix.push(t);
        }

        mix
    }

    fn query_simple(&self, sql_str: &str) -> Vec<SimpleItem> {
        let lib = match &self.sqlite {
            Some(l) => l,
            None => return Vec::new(),
        };
        let c_path = match CString::new(self.db_path.to_string_lossy().as_bytes()) {
            Ok(p) => p,
            Err(_) => return Vec::new(),
        };
        let mut db: *mut c_void = std::ptr::null_mut();
        if unsafe { (lib.open)(c_path.as_ptr(), &mut db) } != SQLITE_OK {
            return Vec::new();
        }

        let mut results = Vec::new();
        let sql = CString::new(sql_str).unwrap();
        let mut stmt: *mut c_void = std::ptr::null_mut();
        if unsafe { (lib.prepare_v2)(db, sql.as_ptr(), -1, &mut stmt, std::ptr::null_mut()) } == SQLITE_OK {
            while unsafe { (lib.step)(stmt) } == SQLITE_ROW {
                let name = self.extract_text(lib, stmt, 0);
                let track_count = unsafe { (lib.column_int)(stmt, 1) as u32 };
                results.push(SimpleItem { name, track_count });
            }
            unsafe { (lib.finalize)(stmt) };
        }
        unsafe { (lib.close)(db) };
        results
    }

    fn extract_text(&self, lib: &SqliteLib, stmt: *mut c_void, col: i32) -> String {
        let ptr = unsafe { (lib.column_text)(stmt, col) };
        if ptr.is_null() {
            String::new()
        } else {
            unsafe { CStr::from_ptr(ptr).to_string_lossy().into_owned() }
        }
    }
}
