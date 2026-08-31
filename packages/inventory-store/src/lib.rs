use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::{
    path::Path,
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum InventoryError {
    #[error("database error")]
    Database(#[from] rusqlite::Error),
    #[error("validation: {0}")]
    Validation(String),
    #[error("not found")]
    NotFound,
    #[error("insufficient stock")]
    InsufficientStock,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub sku: String,
    pub name: String,
    pub unit: String,
    pub category: String,
    pub reorder_level: f64,
    pub active: bool,
    pub created_at: i64,
    pub updated_at: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Supplier {
    pub id: String,
    pub name: String,
    pub contact: Option<String>,
    pub phone: Option<String>,
    pub status: String,
    pub created_at: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Movement {
    pub id: String,
    pub item_id: String,
    pub movement_type: String,
    pub quantity: f64,
    pub unit: String,
    pub from_location: Option<String>,
    pub to_location: Option<String>,
    pub unit_cost_minor: Option<i64>,
    pub supplier_id: Option<String>,
    pub reason: Option<String>,
    pub actor_id: String,
    pub device_id: String,
    pub created_at: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StockLevel {
    pub item_id: String,
    pub sku: String,
    pub name: String,
    pub unit: String,
    pub reorder_level: f64,
    pub on_hand: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StockCount {
    pub id: String,
    pub item_id: String,
    pub location: String,
    pub expected: f64,
    pub counted: f64,
    pub variance: f64,
    pub counted_by: String,
    pub created_at: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Expense {
    pub id: String,
    pub category: String,
    pub amount_minor: i64,
    pub currency: String,
    pub payment_method: String,
    pub status: String,
    pub description: String,
    pub reference: Option<String>,
    pub created_by: String,
    pub approved_by: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
fn id() -> String {
    Uuid::new_v4().to_string()
}
fn need(v: &str, n: &str) -> Result<(), InventoryError> {
    if v.trim().is_empty() {
        return Err(InventoryError::Validation(format!("{n} is required")));
    }
    Ok(())
}

fn row_item(r: &rusqlite::Row<'_>) -> rusqlite::Result<Item> {
    Ok(Item {
        id: r.get(0)?,
        sku: r.get(1)?,
        name: r.get(2)?,
        unit: r.get(3)?,
        category: r.get(4)?,
        reorder_level: r.get(5)?,
        active: r.get::<_, i64>(6)? == 1,
        created_at: r.get(7)?,
        updated_at: r.get(8)?,
    })
}
fn row_supplier(r: &rusqlite::Row<'_>) -> rusqlite::Result<Supplier> {
    Ok(Supplier {
        id: r.get(0)?,
        name: r.get(1)?,
        contact: r.get(2)?,
        phone: r.get(3)?,
        status: r.get(4)?,
        created_at: r.get(5)?,
    })
}
fn row_movement(r: &rusqlite::Row<'_>) -> rusqlite::Result<Movement> {
    Ok(Movement {
        id: r.get(0)?,
        item_id: r.get(1)?,
        movement_type: r.get(2)?,
        quantity: r.get(3)?,
        unit: r.get(4)?,
        from_location: r.get(5)?,
        to_location: r.get(6)?,
        unit_cost_minor: r.get(7)?,
        supplier_id: r.get(8)?,
        reason: r.get(9)?,
        actor_id: r.get(10)?,
        device_id: r.get(11)?,
        created_at: r.get(12)?,
    })
}
fn row_count(r: &rusqlite::Row<'_>) -> rusqlite::Result<StockCount> {
    Ok(StockCount {
        id: r.get(0)?,
        item_id: r.get(1)?,
        location: r.get(2)?,
        expected: r.get(3)?,
        counted: r.get(4)?,
        variance: r.get(5)?,
        counted_by: r.get(6)?,
        created_at: r.get(7)?,
    })
}
fn row_expense(r: &rusqlite::Row<'_>) -> rusqlite::Result<Expense> {
    Ok(Expense {
        id: r.get(0)?,
        category: r.get(1)?,
        amount_minor: r.get(2)?,
        currency: r.get(3)?,
        payment_method: r.get(4)?,
        status: r.get(5)?,
        description: r.get(6)?,
        reference: r.get(7)?,
        created_by: r.get(8)?,
        approved_by: r.get(9)?,
        created_at: r.get(10)?,
        updated_at: r.get(11)?,
    })
}

pub struct InventoryStore {
    conn: Mutex<Connection>,
}

impl InventoryStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, InventoryError> {
        let c = Connection::open(path)?;
        c.pragma_update(None, "journal_mode", "WAL")?;
        c.pragma_update(None, "foreign_keys", "ON")?;
        c.busy_timeout(std::time::Duration::from_secs(5))?;
        let s = Self {
            conn: Mutex::new(c),
        };
        s.migrate()?;
        Ok(s)
    }
    pub fn open_in_memory() -> Result<Self, InventoryError> {
        let c = Connection::open_in_memory()?;
        let s = Self {
            conn: Mutex::new(c),
        };
        s.migrate()?;
        Ok(s)
    }
    fn migrate(&self) -> Result<(), InventoryError> {
        let c = self.conn.lock().unwrap();
        c.execute_batch(
            "CREATE TABLE IF NOT EXISTS items(id TEXT PRIMARY KEY,sku TEXT NOT NULL UNIQUE,name TEXT NOT NULL,unit TEXT NOT NULL,category TEXT NOT NULL,reorder_level REAL NOT NULL DEFAULT 0,active INTEGER NOT NULL DEFAULT 1,created_at INTEGER NOT NULL,updated_at INTEGER NOT NULL);
             CREATE TABLE IF NOT EXISTS suppliers(id TEXT PRIMARY KEY,name TEXT NOT NULL UNIQUE,contact TEXT,phone TEXT,status TEXT NOT NULL DEFAULT 'ACTIVE',created_at INTEGER NOT NULL);
             CREATE TABLE IF NOT EXISTS stock_movements(id TEXT PRIMARY KEY,item_id TEXT NOT NULL REFERENCES items(id),movement_type TEXT NOT NULL,quantity REAL NOT NULL CHECK(quantity>0),unit TEXT NOT NULL,from_location TEXT,to_location TEXT,unit_cost_minor INTEGER,supplier_id TEXT REFERENCES suppliers(id),reason TEXT,actor_id TEXT NOT NULL,device_id TEXT NOT NULL,created_at INTEGER NOT NULL);
             CREATE INDEX IF NOT EXISTS sm_item_loc ON stock_movements(item_id,from_location,to_location);
             CREATE TABLE IF NOT EXISTS stock_counts(id TEXT PRIMARY KEY,item_id TEXT NOT NULL REFERENCES items(id),location TEXT NOT NULL,expected REAL NOT NULL,counted REAL NOT NULL,variance REAL NOT NULL,counted_by TEXT NOT NULL,created_at INTEGER NOT NULL);
             CREATE TABLE IF NOT EXISTS expenses(id TEXT PRIMARY KEY,category TEXT NOT NULL,amount_minor INTEGER NOT NULL CHECK(amount_minor>0),currency TEXT NOT NULL DEFAULT 'UGX',payment_method TEXT NOT NULL,status TEXT NOT NULL DEFAULT 'SUBMITTED' CHECK(status IN ('SUBMITTED','APPROVED','REJECTED')),description TEXT NOT NULL,reference TEXT,created_by TEXT NOT NULL,approved_by TEXT,created_at INTEGER NOT NULL,updated_at INTEGER NOT NULL);",
        )?;
        Ok(())
    }

    pub fn upsert_item(
        &self,
        sku: &str,
        name: &str,
        unit: &str,
        category: &str,
        reorder_level: f64,
    ) -> Result<Item, InventoryError> {
        need(sku, "sku")?;
        need(name, "name")?;
        need(unit, "unit")?;
        need(category, "category")?;
        if reorder_level < 0.0 {
            return Err(InventoryError::Validation(
                "reorder_level must be >= 0".into(),
            ));
        }
        let c = self.conn.lock().unwrap();
        let t = now();
        let existing: Option<String> = c
            .query_row("SELECT id FROM items WHERE sku=?1", params![sku], |r| {
                r.get(0)
            })
            .optional()?;
        let iid = match existing {
            Some(x) => {
                c.execute("UPDATE items SET name=?2,unit=?3,category=?4,reorder_level=?5,updated_at=?6 WHERE id=?1", params![x, name, unit, category, reorder_level, t])?;
                x
            }
            None => {
                let x = id();
                c.execute("INSERT INTO items(id,sku,name,unit,category,reorder_level,active,created_at,updated_at) VALUES(?,?,?,?,?,?,1,?,?)", params![x, sku, name, unit, category, reorder_level, t, t])?;
                x
            }
        };
        c.query_row("SELECT id,sku,name,unit,category,reorder_level,active,created_at,updated_at FROM items WHERE id=?", params![iid], row_item).map_err(Into::into)
    }

    pub fn list_items(&self, active_only: bool) -> Result<Vec<Item>, InventoryError> {
        let c = self.conn.lock().unwrap();
        let mut s = c.prepare("SELECT id,sku,name,unit,category,reorder_level,active,created_at,updated_at FROM items WHERE (?1=0 OR active=1) ORDER BY name")?;
        let mut o = Vec::new();
        for r in s.query_map(params![active_only as i64], row_item)? {
            o.push(r?);
        }
        Ok(o)
    }

    pub fn upsert_supplier(
        &self,
        name: &str,
        contact: Option<&str>,
        phone: Option<&str>,
    ) -> Result<Supplier, InventoryError> {
        need(name, "name")?;
        let c = self.conn.lock().unwrap();
        let t = now();
        let existing: Option<String> = c
            .query_row(
                "SELECT id FROM suppliers WHERE name=?1",
                params![name],
                |r| r.get(0),
            )
            .optional()?;
        let sid = match existing {
            Some(x) => {
                c.execute(
                    "UPDATE suppliers SET contact=?2,phone=?3 WHERE id=?1",
                    params![x, contact, phone],
                )?;
                x
            }
            None => {
                let x = id();
                c.execute("INSERT INTO suppliers(id,name,contact,phone,status,created_at) VALUES(?,?,?,?,'ACTIVE',?)", params![x, name, contact, phone, t])?;
                x
            }
        };
        c.query_row(
            "SELECT id,name,contact,phone,status,created_at FROM suppliers WHERE id=?",
            params![sid],
            row_supplier,
        )
        .map_err(Into::into)
    }

    pub fn list_suppliers(&self) -> Result<Vec<Supplier>, InventoryError> {
        let c = self.conn.lock().unwrap();
        let mut s = c.prepare(
            "SELECT id,name,contact,phone,status,created_at FROM suppliers ORDER BY name",
        )?;
        let mut o = Vec::new();
        for r in s.query_map([], row_supplier)? {
            o.push(r?);
        }
        Ok(o)
    }

    fn balance_at(c: &Connection, item_id: &str, location: &str) -> Result<f64, InventoryError> {
        Ok(c.query_row(
            "SELECT COALESCE(SUM(CASE WHEN to_location=?2 THEN quantity ELSE 0 END),0) - COALESCE(SUM(CASE WHEN from_location=?2 THEN quantity ELSE 0 END),0) FROM stock_movements WHERE item_id=?1",
            params![item_id, location], |r| r.get(0))?)
    }

    pub fn record_movement(
        &self,
        item_id: &str,
        movement_type: &str,
        quantity: f64,
        unit: &str,
        from_location: Option<&str>,
        to_location: Option<&str>,
        unit_cost_minor: Option<i64>,
        supplier_id: Option<&str>,
        reason: Option<&str>,
        actor_id: &str,
        device_id: &str,
    ) -> Result<Movement, InventoryError> {
        need(item_id, "item_id")?;
        need(actor_id, "actor_id")?;
        if !quantity.is_finite() || quantity <= 0.0 {
            return Err(InventoryError::Validation("quantity must be > 0".into()));
        }
        if let Some(uc) = unit_cost_minor {
            if uc < 0 {
                return Err(InventoryError::Validation("unit cost must be >= 0".into()));
            }
        }
        let c = self.conn.lock().unwrap();
        c.query_row("SELECT id FROM items WHERE id=?1", params![item_id], |r| {
            r.get::<_, String>(0)
        })
        .optional()?
        .ok_or(InventoryError::NotFound)?;
        match movement_type {
            "RECEIVE" => {
                if to_location.is_none() {
                    return Err(InventoryError::Validation(
                        "receiving needs a destination".into(),
                    ));
                }
            }
            "ISSUE" => {
                let (f, t) = (from_location, to_location);
                if f.is_none() || t.is_none() || f == t {
                    return Err(InventoryError::Validation(
                        "issue needs differing source and destination".into(),
                    ));
                }
                if Self::balance_at(&c, item_id, f.unwrap())? < quantity {
                    return Err(InventoryError::InsufficientStock);
                }
            }
            "WASTAGE" => {
                if from_location.is_none() {
                    return Err(InventoryError::Validation(
                        "wastage needs a location".into(),
                    ));
                }
                if reason.map(str::trim).unwrap_or("").len() < 3 {
                    return Err(InventoryError::Validation("wastage reason required".into()));
                }
                if Self::balance_at(&c, item_id, from_location.unwrap())? < quantity {
                    return Err(InventoryError::InsufficientStock);
                }
            }
            "ADJUSTMENT" => {
                if from_location.is_none() && to_location.is_none() {
                    return Err(InventoryError::Validation(
                        "adjustment needs a location".into(),
                    ));
                }
                if reason.map(str::trim).unwrap_or("").len() < 3 {
                    return Err(InventoryError::Validation(
                        "adjustment reason required".into(),
                    ));
                }
            }
            other => {
                return Err(InventoryError::Validation(format!(
                    "unknown movement type: {other}"
                )))
            }
        }
        let mid = id();
        let t = now();
        c.execute(
            "INSERT INTO stock_movements(id,item_id,movement_type,quantity,unit,from_location,to_location,unit_cost_minor,supplier_id,reason,actor_id,device_id,created_at) VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?)",
            params![mid, item_id, movement_type, quantity, unit, from_location, to_location, unit_cost_minor, supplier_id, reason, actor_id, device_id, t],
        )?;
        c.query_row("SELECT id,item_id,movement_type,quantity,unit,from_location,to_location,unit_cost_minor,supplier_id,reason,actor_id,device_id,created_at FROM stock_movements WHERE id=?", params![mid], row_movement).map_err(Into::into)
    }

    pub fn stock_levels(&self, location: Option<&str>) -> Result<Vec<StockLevel>, InventoryError> {
        let items = self.list_items(true)?;
        let c = self.conn.lock().unwrap();
        let mut out = Vec::new();
        for i in items {
            let on_hand = match location {
                Some(loc) => Self::balance_at(&c, &i.id, loc)?,
                None => c.query_row(
                    "SELECT COALESCE(SUM(CASE WHEN to_location IS NOT NULL THEN quantity ELSE 0 END),0) - COALESCE(SUM(CASE WHEN from_location IS NOT NULL THEN quantity ELSE 0 END),0) FROM stock_movements WHERE item_id=?1",
                    params![i.id], |r| r.get(0))?,
            };
            out.push(StockLevel {
                item_id: i.id,
                sku: i.sku,
                name: i.name,
                unit: i.unit,
                reorder_level: i.reorder_level,
                on_hand,
            });
        }
        Ok(out)
    }

    pub fn count_stock(
        &self,
        item_id: &str,
        location: &str,
        counted: f64,
        counted_by: &str,
        device_id: &str,
    ) -> Result<StockCount, InventoryError> {
        need(item_id, "item_id")?;
        need(location, "location")?;
        if !counted.is_finite() || counted < 0.0 {
            return Err(InventoryError::Validation("counted must be >= 0".into()));
        }
        let item = {
            let c = self.conn.lock().unwrap();
            c.query_row("SELECT id,sku,name,unit,category,reorder_level,active,created_at,updated_at FROM items WHERE id=?1", params![item_id], row_item).optional()?.ok_or(InventoryError::NotFound)?
        };
        let cid = id();
        let t = now();
        let expected;
        {
            let c = self.conn.lock().unwrap();
            expected = Self::balance_at(&c, item_id, location)?;
            let variance = counted - expected;
            c.execute("INSERT INTO stock_counts(id,item_id,location,expected,counted,variance,counted_by,created_at) VALUES(?,?,?,?,?,?,?,?)", params![cid, item_id, location, expected, counted, variance, counted_by, t])?;
            if variance != 0.0 {
                let (from_loc, to_loc) = if variance > 0.0 {
                    (None, Some(location))
                } else {
                    (Some(location), None)
                };
                c.execute(
                    "INSERT INTO stock_movements(id,item_id,movement_type,quantity,unit,from_location,to_location,unit_cost_minor,supplier_id,reason,actor_id,device_id,created_at) VALUES(?,?,?,?,?,?,?,NULL,NULL,?,?,?,?)",
                    params![id(), item_id, "ADJUSTMENT", variance.abs(), item.unit, from_loc, to_loc, format!("stock count variance (count {cid})"), counted_by, device_id, t],
                )?;
            }
        }
        let c = self.conn.lock().unwrap();
        c.query_row("SELECT id,item_id,location,expected,counted,variance,counted_by,created_at FROM stock_counts WHERE id=?", params![cid], row_count).map_err(Into::into)
    }

    pub fn list_counts(&self, limit: usize) -> Result<Vec<StockCount>, InventoryError> {
        let c = self.conn.lock().unwrap();
        let mut s = c.prepare("SELECT id,item_id,location,expected,counted,variance,counted_by,created_at FROM stock_counts ORDER BY created_at DESC LIMIT ?1")?;
        let mut o = Vec::new();
        for r in s.query_map(params![limit.clamp(1, 500) as i64], row_count)? {
            o.push(r?);
        }
        Ok(o)
    }

    pub fn create_expense(
        &self,
        category: &str,
        amount_minor: i64,
        payment_method: &str,
        description: &str,
        reference: Option<&str>,
        created_by: &str,
    ) -> Result<Expense, InventoryError> {
        need(category, "category")?;
        need(description, "description")?;
        need(created_by, "created_by")?;
        if amount_minor <= 0 {
            return Err(InventoryError::Validation("amount must be > 0".into()));
        }
        let c = self.conn.lock().unwrap();
        let eid = id();
        let t = now();
        c.execute("INSERT INTO expenses(id,category,amount_minor,currency,payment_method,status,description,reference,created_by,created_at,updated_at) VALUES(?,?,?,'UGX',?,'SUBMITTED',?,?,?,?,?)", params![eid, category, amount_minor, payment_method, description, reference, created_by, t, t])?;
        c.query_row("SELECT id,category,amount_minor,currency,payment_method,status,description,reference,created_by,approved_by,created_at,updated_at FROM expenses WHERE id=?", params![eid], row_expense).map_err(Into::into)
    }

    fn decide_expense(
        &self,
        expense_id: &str,
        by: &str,
        decision: &str,
    ) -> Result<Expense, InventoryError> {
        let c = self.conn.lock().unwrap();
        let t = now();
        let n = c.execute("UPDATE expenses SET status=?2,approved_by=?3,updated_at=?4 WHERE id=?1 AND status='SUBMITTED'", params![expense_id, decision, by, t])?;
        if n == 0 {
            return Err(InventoryError::NotFound);
        }
        c.query_row("SELECT id,category,amount_minor,currency,payment_method,status,description,reference,created_by,approved_by,created_at,updated_at FROM expenses WHERE id=?", params![expense_id], row_expense).map_err(Into::into)
    }
    pub fn approve_expense(&self, expense_id: &str, by: &str) -> Result<Expense, InventoryError> {
        self.decide_expense(expense_id, by, "APPROVED")
    }
    pub fn reject_expense(&self, expense_id: &str, by: &str) -> Result<Expense, InventoryError> {
        self.decide_expense(expense_id, by, "REJECTED")
    }

    pub fn list_expenses(
        &self,
        status: Option<&str>,
        limit: usize,
    ) -> Result<Vec<Expense>, InventoryError> {
        let c = self.conn.lock().unwrap();
        let mut s = c.prepare("SELECT id,category,amount_minor,currency,payment_method,status,description,reference,created_by,approved_by,created_at,updated_at FROM expenses WHERE (?1 IS NULL OR status=?1) ORDER BY created_at DESC LIMIT ?2")?;
        let mut o = Vec::new();
        for r in s.query_map(params![status, limit.clamp(1, 500) as i64], row_expense)? {
            o.push(r?);
        }
        Ok(o)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> InventoryStore {
        InventoryStore::open_in_memory().unwrap()
    }

    #[test]
    fn receive_issue_and_wastage_respect_balances() {
        let s = store();
        let item = s
            .upsert_item("FLR-001", "Wheat Flour", "KG", "INGREDIENT", 10.0)
            .unwrap();
        s.record_movement(
            &item.id,
            "RECEIVE",
            50.0,
            "KG",
            None,
            Some("STORE"),
            Some(4200),
            None,
            None,
            "ceo",
            "dev",
        )
        .unwrap();
        s.record_movement(
            &item.id,
            "ISSUE",
            20.0,
            "KG",
            Some("STORE"),
            Some("KITCHEN"),
            None,
            None,
            Some("morning prep"),
            "keeper",
            "dev",
        )
        .unwrap();
        let levels = s.stock_levels(Some("STORE")).unwrap();
        assert_eq!(levels[0].on_hand, 30.0);
        let kitchen = s.stock_levels(Some("KITCHEN")).unwrap();
        assert_eq!(kitchen[0].on_hand, 20.0);
        assert!(matches!(
            s.record_movement(
                &item.id,
                "WASTAGE",
                100.0,
                "KG",
                Some("STORE"),
                None,
                None,
                None,
                Some("spoiled"),
                "keeper",
                "dev"
            ),
            Err(InventoryError::InsufficientStock)
        ));
    }

    #[test]
    fn count_variance_creates_compensating_adjustment() {
        let s = store();
        let item = s
            .upsert_item("OIL-001", "Cooking Oil", "L", "INGREDIENT", 5.0)
            .unwrap();
        s.record_movement(
            &item.id,
            "RECEIVE",
            25.0,
            "L",
            None,
            Some("STORE"),
            None,
            None,
            None,
            "ceo",
            "dev",
        )
        .unwrap();
        let count = s
            .count_stock(&item.id, "STORE", 22.0, "keeper", "dev")
            .unwrap();
        assert_eq!(count.expected, 25.0);
        assert_eq!(count.variance, -3.0);
        assert_eq!(s.stock_levels(Some("STORE")).unwrap()[0].on_hand, 22.0);
    }

    #[test]
    fn expense_approval_flow() {
        let s = store();
        let e = s
            .create_expense(
                "Utilities",
                50000,
                "CASH",
                "Electricity top-up",
                None,
                "manager",
            )
            .unwrap();
        assert_eq!(e.status, "SUBMITTED");
        let approved = s.approve_expense(&e.id, "ceo").unwrap();
        assert_eq!(approved.status, "APPROVED");
        assert!(s.approve_expense(&e.id, "ceo").is_err());
    }
}
