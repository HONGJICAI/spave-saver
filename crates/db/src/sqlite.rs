use crate::models::{DuplicateRecord, FileRecord, ScanRecord};
use anyhow::Result;
use rusqlite::{params, Connection};
use std::path::Path;

/// SQLite database for persistent storage
pub struct SqliteDatabase {
    conn: Connection,
}

impl SqliteDatabase {
    /// Create a new database connection
    pub fn new(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        let db = Self { conn };
        db.init_tables()?;
        Ok(db)
    }

    /// Create an in-memory database (for testing)
    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self { conn };
        db.init_tables()?;
        Ok(db)
    }

    /// Initialize database tables
    fn init_tables(&self) -> Result<()> {
        // Files table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS files (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                path TEXT NOT NULL UNIQUE,
                size INTEGER NOT NULL,
                hash TEXT,
                file_type TEXT NOT NULL,
                modified INTEGER NOT NULL,
                created_at INTEGER NOT NULL
            )",
            [],
        )?;

        // Scans table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS scans (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                path TEXT NOT NULL,
                file_count INTEGER NOT NULL,
                total_size INTEGER NOT NULL,
                scan_time INTEGER NOT NULL,
                created_at INTEGER NOT NULL
            )",
            [],
        )?;

        // Duplicates table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS duplicates (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                hash TEXT NOT NULL,
                file_paths TEXT NOT NULL,
                file_count INTEGER NOT NULL,
                total_size INTEGER NOT NULL,
                wasted_space INTEGER NOT NULL,
                created_at INTEGER NOT NULL
            )",
            [],
        )?;

        // Create indices
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_files_hash ON files(hash)",
            [],
        )?;

        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_files_size ON files(size)",
            [],
        )?;

        Ok(())
    }

    /// Insert a file record
    pub fn insert_file(&self, file: &FileRecord) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO files (path, size, hash, file_type, modified, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                file.path,
                file.size as i64,
                file.hash,
                file.file_type,
                file.modified,
                file.created_at,
            ],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Get file by path
    pub fn get_file_by_path(&self, path: &str) -> Result<Option<FileRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, path, size, hash, file_type, modified, created_at 
             FROM files WHERE path = ?1",
        )?;

        let file = stmt.query_row(params![path], |row| {
            Ok(FileRecord {
                id: row.get(0)?,
                path: row.get(1)?,
                size: row.get::<_, i64>(2)? as u64,
                hash: row.get(3)?,
                file_type: row.get(4)?,
                modified: row.get(5)?,
                created_at: row.get(6)?,
            })
        });

        match file {
            Ok(f) => Ok(Some(f)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Get all files with a specific hash (duplicates)
    pub fn get_files_by_hash(&self, hash: &str) -> Result<Vec<FileRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, path, size, hash, file_type, modified, created_at 
             FROM files WHERE hash = ?1",
        )?;

        let files = stmt.query_map(params![hash], |row| {
            Ok(FileRecord {
                id: row.get(0)?,
                path: row.get(1)?,
                size: row.get::<_, i64>(2)? as u64,
                hash: row.get(3)?,
                file_type: row.get(4)?,
                modified: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;

        let mut result = Vec::new();
        for file in files {
            result.push(file?);
        }

        Ok(result)
    }

    /// Insert a scan record
    pub fn insert_scan(&self, scan: &ScanRecord) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO scans (path, file_count, total_size, scan_time, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                scan.path,
                scan.file_count as i64,
                scan.total_size as i64,
                scan.scan_time,
                scan.created_at,
            ],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Get recent scans
    pub fn get_recent_scans(&self, limit: usize) -> Result<Vec<ScanRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, path, file_count, total_size, scan_time, created_at 
             FROM scans ORDER BY created_at DESC LIMIT ?1",
        )?;

        let scans = stmt.query_map(params![limit], |row| {
            Ok(ScanRecord {
                id: row.get(0)?,
                path: row.get(1)?,
                file_count: row.get::<_, i64>(2)? as usize,
                total_size: row.get::<_, i64>(3)? as u64,
                scan_time: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?;

        let mut result = Vec::new();
        for scan in scans {
            result.push(scan?);
        }

        Ok(result)
    }

    /// Insert a duplicate record
    pub fn insert_duplicate(&self, dup: &DuplicateRecord) -> Result<i64> {
        let file_paths_json = serde_json::to_string(&dup.file_paths)?;

        self.conn.execute(
            "INSERT INTO duplicates (hash, file_paths, file_count, total_size, wasted_space, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                dup.hash,
                file_paths_json,
                dup.file_count as i64,
                dup.total_size as i64,
                dup.wasted_space as i64,
                dup.created_at,
            ],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Get all duplicate groups
    pub fn get_duplicates(&self) -> Result<Vec<DuplicateRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, hash, file_paths, file_count, total_size, wasted_space, created_at 
             FROM duplicates ORDER BY wasted_space DESC",
        )?;

        let dups = stmt.query_map([], |row| {
            let file_paths_json: String = row.get(2)?;
            let file_paths: Vec<String> =
                serde_json::from_str(&file_paths_json).unwrap_or_default();

            Ok(DuplicateRecord {
                id: row.get(0)?,
                hash: row.get(1)?,
                file_paths,
                file_count: row.get::<_, i64>(3)? as usize,
                total_size: row.get::<_, i64>(4)? as u64,
                wasted_space: row.get::<_, i64>(5)? as u64,
                created_at: row.get(6)?,
            })
        })?;

        let mut result = Vec::new();
        for dup in dups {
            result.push(dup?);
        }

        Ok(result)
    }

    /// Delete a file record
    pub fn delete_file(&self, id: i64) -> Result<()> {
        self.conn
            .execute("DELETE FROM files WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// Clear all data
    pub fn clear_all(&self) -> Result<()> {
        self.conn.execute("DELETE FROM files", [])?;
        self.conn.execute("DELETE FROM scans", [])?;
        self.conn.execute("DELETE FROM duplicates", [])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_creation() {
        let _db = SqliteDatabase::in_memory().unwrap();
        // Just ensure it can be created
    }

    #[test]
    fn test_insert_and_get_file() {
        let db = SqliteDatabase::in_memory().unwrap();
        let file = FileRecord::new(
            "/test/file.txt".to_string(),
            1024,
            "text".to_string(),
            12345,
        );

        let id = db.insert_file(&file).unwrap();
        assert!(id > 0);

        let retrieved = db.get_file_by_path("/test/file.txt").unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.path, "/test/file.txt");
        assert_eq!(retrieved.size, 1024);
    }

    #[test]
    fn test_scan_record() {
        let db = SqliteDatabase::in_memory().unwrap();
        let scan = ScanRecord::new("/test".to_string(), 100, 1024000, 5);

        let id = db.insert_scan(&scan).unwrap();
        assert!(id > 0);

        let scans = db.get_recent_scans(10).unwrap();
        assert_eq!(scans.len(), 1);
        assert_eq!(scans[0].path, "/test");
    }
}
