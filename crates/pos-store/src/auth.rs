//! User accounts and argon2 PIN authentication.

use argon2::password_hash::{phc::PasswordHash, PasswordHasher, PasswordVerifier};
use argon2::Argon2;
use pos_domain::{now_unix, Role, User};
use rusqlite::{params, OptionalExtension};

use crate::error::{Result, StoreError};
use crate::Db;

/// Hash a PIN/secret into a PHC string (argon2id, random salt).
pub fn hash_pin(pin: &str) -> Result<String> {
    let hash = Argon2::default().hash_password(pin.as_bytes())?;
    Ok(hash.to_string())
}

/// Verify a PIN against a stored PHC string. Returns `false` on any error.
pub fn verify_pin(pin: &str, phc: &str) -> bool {
    match PasswordHash::new(phc) {
        Ok(parsed) => Argon2::default()
            .verify_password(pin.as_bytes(), &parsed)
            .is_ok(),
        Err(_) => false,
    }
}

fn map_user(row: &rusqlite::Row<'_>) -> rusqlite::Result<User> {
    let role: String = row.get("role")?;
    let is_active: i64 = row.get("is_active")?;
    Ok(User {
        id: row.get("id")?,
        username: row.get("username")?,
        role: Role::parse(&role),
        is_active: is_active != 0,
        created_at: row.get("created_at")?,
    })
}

impl Db {
    pub fn user_count(&self) -> Result<i64> {
        Ok(self
            .conn()
            .query_row("SELECT COUNT(*) FROM users", [], |r| r.get(0))?)
    }

    pub fn has_users(&self) -> Result<bool> {
        Ok(self.user_count()? > 0)
    }

    pub fn create_user(&self, username: &str, pin: &str, role: Role) -> Result<i64> {
        let username = username.trim();
        if username.is_empty() {
            return Err(StoreError::Invalid("username is required".into()));
        }
        if pin.len() < 4 || !pin.chars().all(|c| c.is_ascii_digit()) {
            return Err(StoreError::Invalid("PIN must be at least 4 digits".into()));
        }
        let hash = hash_pin(pin)?;
        self.conn().execute(
            "INSERT INTO users (username, pin_hash, role, is_active, created_at)
             VALUES (?1, ?2, ?3, 1, ?4)",
            params![username, hash, role.as_str(), now_unix()],
        )?;
        Ok(self.conn().last_insert_rowid())
    }

    pub fn get_user(&self, id: i64) -> Result<Option<User>> {
        Ok(self
            .conn()
            .query_row("SELECT * FROM users WHERE id = ?1", params![id], map_user)
            .optional()?)
    }

    pub fn list_users(&self) -> Result<Vec<User>> {
        let mut stmt = self
            .conn()
            .prepare("SELECT * FROM users ORDER BY username COLLATE NOCASE")?;
        let rows = stmt.query_map([], map_user)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Authenticate a username + PIN. Returns the user if the credentials are
    /// valid and the account is active.
    pub fn authenticate(&self, username: &str, pin: &str) -> Result<Option<User>> {
        let found: Option<(User, String)> = self
            .conn()
            .query_row(
                "SELECT * FROM users WHERE username = ?1",
                params![username.trim()],
                |row| {
                    let user = map_user(row)?;
                    let hash: String = row.get("pin_hash")?;
                    Ok((user, hash))
                },
            )
            .optional()?;

        match found {
            Some((user, hash)) if user.is_active && verify_pin(pin, &hash) => Ok(Some(user)),
            _ => Ok(None),
        }
    }

    pub fn set_user_active(&self, id: i64, active: bool) -> Result<()> {
        self.conn().execute(
            "UPDATE users SET is_active = ?2 WHERE id = ?1",
            params![id, active as i64],
        )?;
        Ok(())
    }

    pub fn set_user_pin(&self, id: i64, pin: &str) -> Result<()> {
        if pin.len() < 4 || !pin.chars().all(|c| c.is_ascii_digit()) {
            return Err(StoreError::Invalid("PIN must be at least 4 digits".into()));
        }
        let hash = hash_pin(pin)?;
        self.conn().execute(
            "UPDATE users SET pin_hash = ?2 WHERE id = ?1",
            params![id, hash],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_and_verify_round_trip() {
        let phc = hash_pin("1234").unwrap();
        assert!(verify_pin("1234", &phc));
        assert!(!verify_pin("4321", &phc));
        assert!(!verify_pin("1234", "not-a-hash"));
    }

    #[test]
    fn create_and_authenticate() {
        let db = Db::open_in_memory().unwrap();
        assert!(!db.has_users().unwrap());
        let id = db.create_user("admin", "1234", Role::Admin).unwrap();
        assert!(db.has_users().unwrap());
        assert_eq!(db.user_count().unwrap(), 1);

        let ok = db.authenticate("admin", "1234").unwrap();
        assert_eq!(ok.unwrap().id, id);
        assert!(db.authenticate("admin", "9999").unwrap().is_none());
        assert!(db.authenticate("ghost", "1234").unwrap().is_none());
    }

    #[test]
    fn rejects_weak_pins_and_duplicate_users() {
        let db = Db::open_in_memory().unwrap();
        assert!(db.create_user("a", "12", Role::Cashier).is_err());
        assert!(db.create_user("a", "12ab", Role::Cashier).is_err());
        db.create_user("bob", "1234", Role::Cashier).unwrap();
        assert!(db.create_user("bob", "5678", Role::Cashier).is_err());
    }

    #[test]
    fn deactivated_user_cannot_authenticate() {
        let db = Db::open_in_memory().unwrap();
        let id = db.create_user("carl", "1111", Role::Cashier).unwrap();
        db.set_user_active(id, false).unwrap();
        assert!(db.authenticate("carl", "1111").unwrap().is_none());
    }
}
