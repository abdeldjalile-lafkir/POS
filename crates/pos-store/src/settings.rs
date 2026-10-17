//! Key/value settings, including store legal info printed on receipts.

use pos_domain::StoreInfo;
use rusqlite::{params, OptionalExtension};

use crate::error::Result;
use crate::Db;

pub const KEY_STORE_NAME: &str = "store.name";
pub const KEY_STORE_ADDRESS: &str = "store.address";
pub const KEY_STORE_NIF: &str = "store.nif";
pub const KEY_STORE_NIS: &str = "store.nis";
pub const KEY_STORE_RC: &str = "store.rc";
pub const KEY_STORE_AI: &str = "store.ai";
pub const KEY_STORE_PHONE: &str = "store.phone";
pub const KEY_RECEIPT_FOOTER: &str = "store.receipt_footer";

pub const KEY_PRINTER_TRANSPORT: &str = "printer.transport";
pub const KEY_PRINTER_HOST: &str = "printer.host";
pub const KEY_PRINTER_PORT: &str = "printer.port";
pub const KEY_PRINTER_DEVICE: &str = "printer.device";
pub const KEY_PRINTER_PAPER_WIDTH: &str = "printer.paper_width";
pub const KEY_PRINTER_AUTO_CUT: &str = "printer.auto_cut";
pub const KEY_PRINTER_DRAWER_KICK: &str = "printer.drawer_kick";

pub const KEY_LANGUAGE: &str = "app.language";
pub const KEY_SCAN_MAX_GAP_MS: &str = "scan.max_gap_ms";

impl Db {
    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        Ok(self
            .conn()
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                params![key],
                |r| r.get(0),
            )
            .optional()?)
    }

    pub fn get_setting_or(&self, key: &str, default: &str) -> Result<String> {
        Ok(self
            .get_setting(key)?
            .unwrap_or_else(|| default.to_string()))
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.conn().execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn store_info(&self) -> Result<StoreInfo> {
        Ok(StoreInfo {
            name: self.get_setting_or(KEY_STORE_NAME, "")?,
            address: self.get_setting_or(KEY_STORE_ADDRESS, "")?,
            nif: self.get_setting_or(KEY_STORE_NIF, "")?,
            nis: self.get_setting_or(KEY_STORE_NIS, "")?,
            rc: self.get_setting_or(KEY_STORE_RC, "")?,
            ai: self.get_setting_or(KEY_STORE_AI, "")?,
            phone: self.get_setting_or(KEY_STORE_PHONE, "")?,
            receipt_footer: self.get_setting_or(KEY_RECEIPT_FOOTER, "Thank you for your visit")?,
        })
    }

    pub fn save_store_info(&self, info: &StoreInfo) -> Result<()> {
        self.set_setting(KEY_STORE_NAME, &info.name)?;
        self.set_setting(KEY_STORE_ADDRESS, &info.address)?;
        self.set_setting(KEY_STORE_NIF, &info.nif)?;
        self.set_setting(KEY_STORE_NIS, &info.nis)?;
        self.set_setting(KEY_STORE_RC, &info.rc)?;
        self.set_setting(KEY_STORE_AI, &info.ai)?;
        self.set_setting(KEY_STORE_PHONE, &info.phone)?;
        self.set_setting(KEY_RECEIPT_FOOTER, &info.receipt_footer)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_store_info() {
        let db = Db::open_in_memory().unwrap();
        let info = StoreInfo {
            name: "Superette El Baraka".into(),
            address: "12 Rue Didouche Mourad, Alger".into(),
            nif: "000123456789012".into(),
            nis: "000123456789013".into(),
            rc: "16/00-1234567 B 12".into(),
            ai: "16050123456".into(),
            phone: "+213 21 00 00 00".into(),
            receipt_footer: "Merci".into(),
        };
        db.save_store_info(&info).unwrap();
        let loaded = db.store_info().unwrap();
        assert_eq!(loaded, info);
    }

    #[test]
    fn missing_settings_fall_back() {
        let db = Db::open_in_memory().unwrap();
        assert_eq!(db.get_setting_or("nope", "def").unwrap(), "def");
    }
}
