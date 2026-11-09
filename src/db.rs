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

    fn find_db_path() -> PathBuf {
        let candidates = [
            PathBuf::from("library.db"),
            PathBuf::from("sonixlink_library.db"),
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|p| p.join("library.db")))
                .unwrap_or_else(|| PathBuf::from("library.db")),
        ];
        for c in &candidates {
            if c.exists() {
                return c.clone();
            }
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
        let query = "SELECT title, artist, album, path, \
                     (CASE WHEN end_time > begin_time THEN (end_time - begin_time) / 1000 ELSE 0 END) AS duration, \
                     ck_id AS track_num \
                     FROM MEDIA_TABLE WHERE path <> '' ORDER BY sortkey, title LIMIT 5000";
        self.query_tracks(query)
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
