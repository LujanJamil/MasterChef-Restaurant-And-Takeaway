use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::{path::Path, sync::Mutex};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum OrdersError {
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("invalid input: {0}")]
    Invalid(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("invalid order transition from {from} to {to}")]
    InvalidTransition { from: String, to: String },
    #[error("item is not available for {0} orders")]
    NotAvailable(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("corrupt data: {0}")]
    Corrupt(String),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum OrderType {
    DineIn,
    Takeaway,
    Delivery,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum OrderStatus {
    Draft,
    Submitted,
    Preparing,
    Ready,
    Served,
    Completed,
    Cancelled,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ItemStatus {
    Ordered,
    Preparing,
    Ready,
    Served,
    Cancelled,
}

impl OrderType {
    fn s(&self) -> &'static str {
        match self {
            Self::DineIn => "DINE_IN",
            Self::Takeaway => "TAKEAWAY",
            Self::Delivery => "DELIVERY",
        }
    }
    /// Strict decode: unknown persisted values indicate database corruption.
    fn from_db(v: &str) -> Result<Self, OrdersError> {
        match v {
            "DINE_IN" => Ok(Self::DineIn),
            "TAKEAWAY" => Ok(Self::Takeaway),
            "DELIVERY" => Ok(Self::Delivery),
            other => Err(OrdersError::Corrupt(format!(
                "unknown order_type '{other}'"
            ))),
        }
    }
}
impl OrderStatus {
    fn s(&self) -> &'static str {
        match self {
            Self::Draft => "DRAFT",
            Self::Submitted => "SUBMITTED",
            Self::Preparing => "PREPARING",
            Self::Ready => "READY",
            Self::Served => "SERVED",
            Self::Completed => "COMPLETED",
            Self::Cancelled => "CANCELLED",
        }
    }
    /// Strict decode: unknown persisted values indicate database corruption.
    fn from_db(v: &str) -> Result<Self, OrdersError> {
        match v {
            "DRAFT" => Ok(Self::Draft),
            "SUBMITTED" => Ok(Self::Submitted),
            "PREPARING" => Ok(Self::Preparing),
            "READY" => Ok(Self::Ready),
            "SERVED" => Ok(Self::Served),
            "COMPLETED" => Ok(Self::Completed),
            "CANCELLED" => Ok(Self::Cancelled),
            other => Err(OrdersError::Corrupt(format!(
                "unknown order status '{other}'"
            ))),
        }
    }
}
impl ItemStatus {
    fn s(&self) -> &'static str {
        match self {
            Self::Ordered => "ORDERED",
            Self::Preparing => "PREPARING",
            Self::Ready => "READY",
            Self::Served => "SERVED",
            Self::Cancelled => "CANCELLED",
        }
    }
    /// Strict decode: unknown persisted values indicate database corruption.
    fn from_db(v: &str) -> Result<Self, OrdersError> {
        match v {
            "ORDERED" => Ok(Self::Ordered),
            "PREPARING" => Ok(Self::Preparing),
            "READY" => Ok(Self::Ready),
            "SERVED" => Ok(Self::Served),
            "CANCELLED" => Ok(Self::Cancelled),
            other => Err(OrdersError::Corrupt(format!(
                "unknown item status '{other}'"
            ))),
        }
    }
}

/// Upper bound for a single order line quantity; guards against absurd
/// allocations/overflows from untrusted frontend input.
const MAX_LINE_QUANTITY: i64 = 10_000;

fn corrupt_err(e: OrdersError) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
}

/// Normalize and validate an ISO-style currency code: exactly three ASCII
/// letters, stored uppercase (e.g. "ugx" -> "UGX"). Rejects values like
/// "@@@", "123" or "ab".
fn normalize_currency(currency: &str) -> Result<String, OrdersError> {
    let c = currency.trim().to_ascii_uppercase();
    if c.len() == 3 && c.chars().all(|ch| ch.is_ascii_uppercase()) {
        Ok(c)
    } else {
        Err(OrdersError::Invalid(format!(
            "invalid currency code '{currency}': expected three letters (e.g. UGX)"
        )))
    }
}

/// True when a rusqlite error is a constraint violation. The category and
/// restaurant_tables inserts only have UNIQUE constraints, so any
/// constraint violation there means a duplicate name.
fn is_unique_violation(e: &rusqlite::Error) -> bool {
    matches!(
        e,
        rusqlite::Error::SqliteFailure(se, _)
            if se.code == rusqlite::ErrorCode::ConstraintViolation
    )
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: String,
    pub name: String,
    pub active: bool,
    pub sort_order: i64,
    pub created_at: i64,
    pub updated_at: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenuItem {
    pub id: String,
    pub category_id: String,
    pub name: String,
    pub description: Option<String>,
    pub station: String,
    pub active: bool,
    pub dine_in: bool,
    pub takeaway: bool,
    pub delivery: bool,
    pub current_price_minor: i64,
    pub currency: String,
    pub created_at: i64,
    pub updated_at: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table {
    pub id: String,
    pub name: String,
    pub active: bool,
    pub created_at: i64,
    pub updated_at: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    /// Globally unique, immutable entity identity used for synchronization.
    pub id: String,
    /// Local, human-facing business sequence. NOT a global identity: offline
    /// devices can allocate the same number independently; the backend is
    /// responsible for any global numbering on synchronization.
    pub order_number: i64,
    pub order_type: OrderType,
    pub table_id: Option<String>,
    pub customer_name: Option<String>,
    pub customer_phone: Option<String>,
    pub waiter_id: String,
    pub status: OrderStatus,
    pub subtotal_minor: i64,
    pub discount_minor: i64,
    pub total_minor: i64,
    pub currency: String,
    pub notes: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderItem {
    pub id: String,
    pub order_id: String,
    pub menu_item_id: String,
    /// Immutable snapshot of the menu item name at order time.
    pub name_snapshot: String,
    /// Immutable snapshot of the unit price at order time; later menu price
    /// changes never rewrite historical order lines.
    pub unit_price_minor: i64,
    pub quantity: i64,
    pub total_minor: i64,
    pub station: String,
    pub status: ItemStatus,
    pub notes: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderEvent {
    pub id: String,
    pub order_id: String,
    pub item_id: Option<String>,
    pub actor_id: String,
    pub event_type: String,
    pub reason: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: i64,
}

/// Order/menu store. The connection lives behind a `Mutex` (matching the
/// other core packages), which keeps the store `Sync` for Tauri state
/// management and serializes the multi-statement transactions below so two
/// writers on the same connection can never interleave.
pub struct OrdersStore {
    conn: Mutex<Connection>,
}

impl OrdersStore {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, OrdersError> {
        let c = Connection::open(path)?;
        // Local POS reliability settings (match the other core packages):
        // WAL for concurrent readers, NORMAL synchronous is the recommended
        // pairing for WAL durability, foreign keys enforced.
        c.pragma_update(None, "journal_mode", "WAL")?;
        c.pragma_update(None, "synchronous", "NORMAL")?;
        c.pragma_update(None, "foreign_keys", "ON")?;
        c.busy_timeout(std::time::Duration::from_secs(5))?;
        let s = Self {
            conn: Mutex::new(c),
        };
        s.migrate()?;
        Ok(s)
    }

    /// Versioned, non-destructive migrations. `user_version` tracks which
    /// migrations have been applied so existing databases keep their data.
    fn migrate(&self) -> Result<(), OrdersError> {
        let conn = self.conn.lock().expect("orders store mutex poisoned");
        let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if version < 1 {
            // Fresh database: create the base schema. Gated on user_version
            // so re-opening an existing database never regresses the version
            // or re-runs migrations.
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS categories(id TEXT PRIMARY KEY,name TEXT NOT NULL,active INTEGER NOT NULL DEFAULT 1,sort_order INTEGER NOT NULL DEFAULT 0,created_at INTEGER NOT NULL,updated_at INTEGER NOT NULL);
             CREATE UNIQUE INDEX IF NOT EXISTS categories_name_ci ON categories(lower(name));
             CREATE TABLE IF NOT EXISTS menu_items(id TEXT PRIMARY KEY,category_id TEXT NOT NULL REFERENCES categories(id),name TEXT NOT NULL,description TEXT,station TEXT NOT NULL,active INTEGER NOT NULL DEFAULT 1,dine_in INTEGER NOT NULL DEFAULT 1,takeaway INTEGER NOT NULL DEFAULT 1,delivery INTEGER NOT NULL DEFAULT 0,current_price_minor INTEGER NOT NULL CHECK(current_price_minor>=0),currency TEXT NOT NULL DEFAULT 'UGX',created_at INTEGER NOT NULL,updated_at INTEGER NOT NULL);
             CREATE INDEX IF NOT EXISTS menu_category_active ON menu_items(category_id,active);
             CREATE TABLE IF NOT EXISTS menu_prices(id TEXT PRIMARY KEY,menu_item_id TEXT NOT NULL REFERENCES menu_items(id),price_minor INTEGER NOT NULL CHECK(price_minor>=0),currency TEXT NOT NULL,effective_from INTEGER NOT NULL,effective_to INTEGER,created_by TEXT NOT NULL,created_at INTEGER NOT NULL);
             CREATE INDEX IF NOT EXISTS menu_price_effective ON menu_prices(menu_item_id,effective_from);
             CREATE TABLE IF NOT EXISTS restaurant_tables(id TEXT PRIMARY KEY,name TEXT NOT NULL UNIQUE,active INTEGER NOT NULL DEFAULT 1,created_at INTEGER NOT NULL,updated_at INTEGER NOT NULL);
             CREATE TABLE IF NOT EXISTS orders(id TEXT PRIMARY KEY,order_number INTEGER NOT NULL UNIQUE,order_type TEXT NOT NULL CHECK(order_type IN ('DINE_IN','TAKEAWAY','DELIVERY')),table_id TEXT REFERENCES restaurant_tables(id),customer_name TEXT,customer_phone TEXT,waiter_id TEXT NOT NULL,status TEXT NOT NULL,subtotal_minor INTEGER NOT NULL DEFAULT 0 CHECK(subtotal_minor>=0),discount_minor INTEGER NOT NULL DEFAULT 0 CHECK(discount_minor>=0),total_minor INTEGER NOT NULL DEFAULT 0 CHECK(total_minor>=0),currency TEXT NOT NULL DEFAULT 'UGX',notes TEXT,created_at INTEGER NOT NULL,updated_at INTEGER NOT NULL,CHECK(total_minor=subtotal_minor-discount_minor),CHECK(order_type<>'DINE_IN' OR table_id IS NOT NULL));
             CREATE INDEX IF NOT EXISTS orders_status ON orders(status,updated_at);
             CREATE TABLE IF NOT EXISTS order_items(id TEXT PRIMARY KEY,order_id TEXT NOT NULL REFERENCES orders(id) ON DELETE CASCADE,menu_item_id TEXT NOT NULL REFERENCES menu_items(id),name_snapshot TEXT NOT NULL,unit_price_minor INTEGER NOT NULL CHECK(unit_price_minor>=0),quantity INTEGER NOT NULL CHECK(quantity>0),total_minor INTEGER NOT NULL CHECK(total_minor>=0),station TEXT NOT NULL,status TEXT NOT NULL,notes TEXT,created_at INTEGER NOT NULL,updated_at INTEGER NOT NULL,CHECK(total_minor=unit_price_minor*quantity));
             CREATE INDEX IF NOT EXISTS order_items_order ON order_items(order_id);
             CREATE TABLE IF NOT EXISTS order_events(id TEXT PRIMARY KEY,order_id TEXT NOT NULL REFERENCES orders(id),item_id TEXT,actor_id TEXT NOT NULL,event_type TEXT NOT NULL,reason TEXT,metadata_json TEXT NOT NULL,created_at INTEGER NOT NULL);
             CREATE INDEX IF NOT EXISTS order_events_order ON order_events(order_id,created_at);
             CREATE TABLE IF NOT EXISTS order_number_sequence(id INTEGER PRIMARY KEY CHECK(id=1),next_number INTEGER NOT NULL);
             INSERT OR IGNORE INTO order_number_sequence(id,next_number) VALUES(1,1);
             PRAGMA user_version=1;",
            )?;
        }

        // Migration 2: enforce valid status values at the database level.
        // Existing data is validated first; if any row violates the new
        // CHECK constraint the migration is aborted instead of destroying
        // data, leaving the pre-existing (unconstrained) schema in place.
        if version < 2 {
            Self::migrate_v2_status_constraints(&conn)?;
        }
        Ok(())
    }

    fn migrate_v2_status_constraints(conn: &Connection) -> Result<(), OrdersError> {
        let bad_orders: i64 = conn.query_row(
            "SELECT COUNT(*) FROM orders WHERE status NOT IN ('DRAFT','SUBMITTED','PREPARING','READY','SERVED','COMPLETED','CANCELLED')",
            [],
            |r| r.get(0),
        )?;
        if bad_orders > 0 {
            return Err(OrdersError::Corrupt(format!(
                "{bad_orders} order(s) contain invalid status values; refusing migration"
            )));
        }
        let bad_items: i64 = conn.query_row(
            "SELECT COUNT(*) FROM order_items WHERE status NOT IN ('ORDERED','PREPARING','READY','SERVED','CANCELLED')",
            [],
            |r| r.get(0),
        )?;
        if bad_items > 0 {
            return Err(OrdersError::Corrupt(format!(
                "{bad_items} order item(s) contain invalid status values; refusing migration"
            )));
        }
        // The store mutex is already held here, so no other writer can touch
        // this connection while the rebuild runs.
        let tx = conn.unchecked_transaction()?;
        // 12-step table rebuild preserving ids, indexes and data.
        tx.execute_batch(
            "CREATE TABLE orders_new(id TEXT PRIMARY KEY,order_number INTEGER NOT NULL UNIQUE,order_type TEXT NOT NULL CHECK(order_type IN ('DINE_IN','TAKEAWAY','DELIVERY')),table_id TEXT REFERENCES restaurant_tables(id),customer_name TEXT,customer_phone TEXT,waiter_id TEXT NOT NULL,status TEXT NOT NULL CHECK(status IN ('DRAFT','SUBMITTED','PREPARING','READY','SERVED','COMPLETED','CANCELLED')),subtotal_minor INTEGER NOT NULL DEFAULT 0 CHECK(subtotal_minor>=0),discount_minor INTEGER NOT NULL DEFAULT 0 CHECK(discount_minor>=0),total_minor INTEGER NOT NULL DEFAULT 0 CHECK(total_minor>=0),currency TEXT NOT NULL DEFAULT 'UGX',notes TEXT,created_at INTEGER NOT NULL,updated_at INTEGER NOT NULL,CHECK(total_minor=subtotal_minor-discount_minor),CHECK(order_type<>'DINE_IN' OR table_id IS NOT NULL));
             INSERT INTO orders_new(id,order_number,order_type,table_id,customer_name,customer_phone,waiter_id,status,subtotal_minor,discount_minor,total_minor,currency,notes,created_at,updated_at) SELECT id,order_number,order_type,table_id,customer_name,customer_phone,waiter_id,status,subtotal_minor,discount_minor,total_minor,currency,notes,created_at,updated_at FROM orders;
             DROP TABLE orders;
             ALTER TABLE orders_new RENAME TO orders;
             CREATE INDEX IF NOT EXISTS orders_status ON orders(status,updated_at);
             CREATE TABLE order_items_new(id TEXT PRIMARY KEY,order_id TEXT NOT NULL REFERENCES orders(id) ON DELETE CASCADE,menu_item_id TEXT NOT NULL REFERENCES menu_items(id),name_snapshot TEXT NOT NULL,unit_price_minor INTEGER NOT NULL CHECK(unit_price_minor>=0),quantity INTEGER NOT NULL CHECK(quantity>0),total_minor INTEGER NOT NULL CHECK(total_minor>=0),station TEXT NOT NULL,status TEXT NOT NULL CHECK(status IN ('ORDERED','PREPARING','READY','SERVED','CANCELLED')),notes TEXT,created_at INTEGER NOT NULL,updated_at INTEGER NOT NULL,CHECK(total_minor=unit_price_minor*quantity));
             INSERT INTO order_items_new(id,order_id,menu_item_id,name_snapshot,unit_price_minor,quantity,total_minor,station,status,notes,created_at,updated_at) SELECT id,order_id,menu_item_id,name_snapshot,unit_price_minor,quantity,total_minor,station,status,notes,created_at,updated_at FROM order_items;
             DROP TABLE order_items;
             ALTER TABLE order_items_new RENAME TO order_items;
             CREATE INDEX IF NOT EXISTS order_items_order ON order_items(order_id);
             PRAGMA user_version=2;",
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn create_category(
        &self,
        name: &str,
        sort: i64,
        now: i64,
    ) -> Result<Category, OrdersError> {
        let n = name.trim();
        if n.is_empty() {
            return Err(OrdersError::Invalid("category name is required".into()));
        }
        let id = Uuid::new_v4().to_string();
        let conn = self.conn.lock().expect("orders store mutex poisoned");
        conn.execute(
            "INSERT INTO categories(id,name,sort_order,created_at,updated_at) VALUES(?,?,?,?,?)",
            params![id, n, sort, now, now],
        )
        .map_err(|e| {
            if is_unique_violation(&e) {
                OrdersError::Conflict(format!("a category named '{n}' already exists"))
            } else {
                OrdersError::Db(e)
            }
        })?;
        Ok(Category {
            id,
            name: n.into(),
            active: true,
            sort_order: sort,
            created_at: now,
            updated_at: now,
        })
    }

    pub fn list_categories(&self) -> Result<Vec<Category>, OrdersError> {
        let conn = self.conn.lock().expect("orders store mutex poisoned");
        let mut s = conn.prepare(
            "SELECT id,name,active,sort_order,created_at,updated_at FROM categories ORDER BY sort_order,name",
        )?;
        let result = s
            .query_map([], |r| {
                Ok(Category {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    active: r.get::<_, i64>(2)? != 0,
                    sort_order: r.get(3)?,
                    created_at: r.get(4)?,
                    updated_at: r.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(result)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_menu_item(
        &self,
        cat: &str,
        name: &str,
        desc: Option<&str>,
        station: &str,
        price: i64,
        currency: &str,
        di: bool,
        to: bool,
        de: bool,
        actor: &str,
        now: i64,
    ) -> Result<MenuItem, OrdersError> {
        if price < 0 {
            return Err(OrdersError::Invalid("price cannot be negative".into()));
        }
        let currency = normalize_currency(currency)?;
        let n = name.trim();
        let st = station.trim();
        if n.is_empty() {
            return Err(OrdersError::Invalid("menu item name is required".into()));
        }
        if st.is_empty() {
            return Err(OrdersError::Invalid("station is required".into()));
        }
        let conn = self.conn.lock().expect("orders store mutex poisoned");
        if conn
            .query_row(
                "SELECT 1 FROM categories WHERE id=? AND active=1",
                params![cat],
                |r| r.get::<_, i64>(0),
            )
            .optional()?
            .is_none()
        {
            return Err(OrdersError::NotFound("category".into()));
        }
        let id = Uuid::new_v4().to_string();
        // The item row and its initial price-history row must commit together.
        // The mutex serializes writers, so no other writer can interleave.
        let tx = conn.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO menu_items(id,category_id,name,description,station,active,current_price_minor,currency,dine_in,takeaway,delivery,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?)",
            params![
                id,
                cat,
                n,
                desc.map(str::trim),
                st,
                1,
                price,
                currency,
                di as i64,
                to as i64,
                de as i64,
                now,
                now
            ],
        )?;
        tx.execute(
            "INSERT INTO menu_prices(id,menu_item_id,price_minor,currency,effective_from,created_by,created_at) VALUES(?,?,?,?,?,?,?)",
            params![Uuid::new_v4().to_string(), id, price, currency, now, actor, now],
        )?;
        tx.commit()?;
        Ok(MenuItem {
            id,
            category_id: cat.into(),
            name: n.into(),
            description: desc.map(|d| d.trim().to_string()),
            station: st.into(),
            active: true,
            dine_in: di,
            takeaway: to,
            delivery: de,
            current_price_minor: price,
            currency,
            created_at: now,
            updated_at: now,
        })
    }

    pub fn list_menu(&self, cat: Option<&str>) -> Result<Vec<MenuItem>, OrdersError> {
        let conn = self.conn.lock().expect("orders store mutex poisoned");
        let mut s = conn.prepare(
            "SELECT id,category_id,name,description,station,active,dine_in,takeaway,delivery,current_price_minor,currency,created_at,updated_at FROM menu_items WHERE active=1 AND (?1 IS NULL OR category_id=?1) ORDER BY name",
        )?;
        let result = s
            .query_map(params![cat], |r| {
                Ok(MenuItem {
                    id: r.get(0)?,
                    category_id: r.get(1)?,
                    name: r.get(2)?,
                    description: r.get(3)?,
                    station: r.get(4)?,
                    active: r.get::<_, i64>(5)? != 0,
                    dine_in: r.get::<_, i64>(6)? != 0,
                    takeaway: r.get::<_, i64>(7)? != 0,
                    delivery: r.get::<_, i64>(8)? != 0,
                    current_price_minor: r.get(9)?,
                    currency: r.get(10)?,
                    created_at: r.get(11)?,
                    updated_at: r.get(12)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(result)
    }

    pub fn create_table(&self, name: &str, now: i64) -> Result<Table, OrdersError> {
        let n = name.trim();
        if n.is_empty() {
            return Err(OrdersError::Invalid("table name is required".into()));
        }
        let id = Uuid::new_v4().to_string();
        let conn = self.conn.lock().expect("orders store mutex poisoned");
        conn.execute(
            "INSERT INTO restaurant_tables(id,name,created_at,updated_at) VALUES(?,?,?,?)",
            params![id, n, now, now],
        )
        .map_err(|e| {
            if is_unique_violation(&e) {
                OrdersError::Conflict(format!("a table named '{n}' already exists"))
            } else {
                OrdersError::Db(e)
            }
        })?;
        Ok(Table {
            id,
            name: n.into(),
            active: true,
            created_at: now,
            updated_at: now,
        })
    }

    pub fn list_tables(&self) -> Result<Vec<Table>, OrdersError> {
        let conn = self.conn.lock().expect("orders store mutex poisoned");
        let mut s = conn.prepare(
            "SELECT id,name,active,created_at,updated_at FROM restaurant_tables WHERE active=1 ORDER BY name",
        )?;
        let result = s
            .query_map([], |r| {
                Ok(Table {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    active: r.get::<_, i64>(2)? != 0,
                    created_at: r.get(3)?,
                    updated_at: r.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(result)
    }

    /// Create an order and allocate its local order number atomically.
    ///
    /// NOTE (multi-device design): `id` (UUID) is the globally unique,
    /// immutable synchronization identity. `order_number` is a per-device
    /// local business/display sequence only; independent offline devices may
    /// allocate the same number, and any global numbering must be assigned by
    /// the backend during synchronization.
    #[allow(clippy::too_many_arguments)]
    pub fn create_order(
        &self,
        k: OrderType,
        table: Option<&str>,
        cn: Option<&str>,
        cp: Option<&str>,
        waiter: &str,
        notes: Option<&str>,
        currency: &str,
        now: i64,
    ) -> Result<Order, OrdersError> {
        if waiter.trim().is_empty() {
            return Err(OrdersError::Invalid("waiter is required".into()));
        }
        let currency = normalize_currency(currency)?;
        if k == OrderType::DineIn && table.is_none() {
            return Err(OrdersError::Invalid(
                "dine-in orders require a table".into(),
            ));
        }
        let cn = cn.map(str::trim).filter(|s| !s.is_empty());
        let cp = cp.map(str::trim).filter(|s| !s.is_empty());
        let notes = notes.map(str::trim).filter(|s| !s.is_empty());

        // BEGIN IMMEDIATE takes the SQLite write lock up front so the
        // read-then-increment of the sequence cannot interleave with a writer
        // on another connection (separate process). Writers on this same
        // connection are serialized by the store mutex. Order + sequence
        // bump + audit event commit or roll back together.
        let mut conn = self.conn.lock().expect("orders store mutex poisoned");
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        if let Some(t) = table {
            if tx
                .query_row(
                    "SELECT 1 FROM restaurant_tables WHERE id=? AND active=1",
                    params![t],
                    |r| r.get::<_, i64>(0),
                )
                .optional()?
                .is_none()
            {
                return Err(OrdersError::NotFound("table".into()));
            }
        }
        let next: i64 = tx.query_row(
            "SELECT next_number FROM order_number_sequence WHERE id=1",
            [],
            |r| r.get(0),
        )?;
        tx.execute(
            "UPDATE order_number_sequence SET next_number=next_number+1 WHERE id=1",
            [],
        )?;
        let id = Uuid::new_v4().to_string();
        tx.execute(
            "INSERT INTO orders(id,order_number,order_type,table_id,customer_name,customer_phone,waiter_id,status,currency,notes,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?,?,?,?,?)",
            params![id, next, k.s(), table, cn, cp, waiter, "DRAFT", currency, notes, now, now],
        )?;
        event_on(&tx, &id, None, waiter, "ORDER_CREATED", None, "{}", now)?;
        tx.commit()?;
        Ok(Order {
            id,
            order_number: next,
            order_type: k,
            table_id: table.map(str::to_string),
            customer_name: cn.map(str::to_string),
            customer_phone: cp.map(str::to_string),
            waiter_id: waiter.into(),
            status: OrderStatus::Draft,
            subtotal_minor: 0,
            discount_minor: 0,
            total_minor: 0,
            currency,
            notes: notes.map(str::to_string),
            created_at: now,
            updated_at: now,
        })
    }

    pub fn add_item(
        &self,
        oid: &str,
        mid: &str,
        q: i64,
        notes: Option<&str>,
        actor: &str,
        now: i64,
    ) -> Result<OrderItem, OrdersError> {
        if q <= 0 {
            return Err(OrdersError::Invalid("quantity must be positive".into()));
        }
        if q > MAX_LINE_QUANTITY {
            return Err(OrdersError::Invalid(
                "quantity is unreasonably large".into(),
            ));
        }
        // Insert item, recalculate totals and append the audit event in a
        // single transaction so the database is never left half-updated.
        let mut conn = self.conn.lock().expect("orders store mutex poisoned");
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let st: String = tx
            .query_row("SELECT status FROM orders WHERE id=?", params![oid], |r| {
                r.get(0)
            })
            .optional()?
            .ok_or_else(|| OrdersError::NotFound("order".into()))?;
        let status = OrderStatus::from_db(&st)?;
        if status != OrderStatus::Draft && status != OrderStatus::Submitted {
            return Err(OrdersError::Invalid(
                "items cannot be changed in this state".into(),
            ));
        }
        let order_type = OrderType::from_db(&tx.query_row(
            "SELECT order_type FROM orders WHERE id=?",
            params![oid],
            |r| r.get::<_, String>(0),
        )?)?;
        let (name, price, station, dine_in, takeaway, delivery): (
            String,
            i64,
            String,
            bool,
            bool,
            bool,
        ) = tx
            .query_row(
                "SELECT name,current_price_minor,station,dine_in,takeaway,delivery FROM menu_items WHERE id=? AND active=1",
                params![mid],
                |r| {
                    Ok((
                        r.get(0)?,
                        r.get(1)?,
                        r.get(2)?,
                        r.get::<_, i64>(3)? != 0,
                        r.get::<_, i64>(4)? != 0,
                        r.get::<_, i64>(5)? != 0,
                    ))
                },
            )
            .optional()?
            .ok_or_else(|| OrdersError::NotFound("menu item".into()))?;
        // Menu availability must match the order's channel.
        let allowed = match order_type {
            OrderType::DineIn => dine_in,
            OrderType::Takeaway => takeaway,
            OrderType::Delivery => delivery,
        };
        if !allowed {
            return Err(OrdersError::NotAvailable(order_type.s().into()));
        }
        let total = price
            .checked_mul(q)
            .ok_or_else(|| OrdersError::Invalid("line total overflow".into()))?;
        let id = Uuid::new_v4().to_string();
        let notes = notes.map(str::trim).filter(|s| !s.is_empty());
        tx.execute(
            "INSERT INTO order_items(id,order_id,menu_item_id,name_snapshot,unit_price_minor,quantity,total_minor,station,status,notes,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?,?,?,?,?)",
            params![id, oid, mid, name, price, q, total, station, ItemStatus::Ordered.s(), notes, now, now],
        )?;
        recalc_on(&tx, oid, now)?;
        event_on(
            &tx,
            oid,
            Some(&id),
            actor,
            "ITEM_ADDED",
            None,
            &serde_json::json!({"quantity": q}).to_string(),
            now,
        )?;
        tx.commit()?;
        Ok(OrderItem {
            id,
            order_id: oid.into(),
            menu_item_id: mid.into(),
            name_snapshot: name,
            unit_price_minor: price,
            quantity: q,
            total_minor: total,
            station,
            status: ItemStatus::Ordered,
            notes: notes.map(str::to_string),
            created_at: now,
            updated_at: now,
        })
    }

    pub fn cancel_item(
        &self,
        iid: &str,
        actor: &str,
        reason: &str,
        now: i64,
    ) -> Result<Order, OrdersError> {
        let reason = reason.trim();
        if reason.is_empty() {
            return Err(OrdersError::Invalid(
                "cancellation reason is required".into(),
            ));
        }
        // Cancel item + recalculate totals + audit event are atomic.
        let mut conn = self.conn.lock().expect("orders store mutex poisoned");
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let (oid, st): (String, String) = tx
            .query_row(
                "SELECT order_id,status FROM order_items WHERE id=?",
                params![iid],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?
            .ok_or_else(|| OrdersError::NotFound("order item".into()))?;
        if ItemStatus::from_db(&st)? == ItemStatus::Cancelled {
            return Err(OrdersError::Invalid("item already cancelled".into()));
        }
        tx.execute(
            "UPDATE order_items SET status=?,updated_at=? WHERE id=?",
            params![ItemStatus::Cancelled.s(), now, iid],
        )?;
        recalc_on(&tx, &oid, now)?;
        event_on(
            &tx,
            &oid,
            Some(iid),
            actor,
            "ITEM_CANCELLED",
            Some(reason),
            "{}",
            now,
        )?;
        tx.commit()?;
        get_order_on(&conn, &oid)
    }

    pub fn set_discount(
        &self,
        oid: &str,
        d: i64,
        actor: &str,
        reason: &str,
        now: i64,
    ) -> Result<Order, OrdersError> {
        let reason = reason.trim();
        if d < 0 {
            return Err(OrdersError::Invalid("discount cannot be negative".into()));
        }
        if reason.is_empty() {
            return Err(OrdersError::Invalid("discount reason is required".into()));
        }
        // Discount update + audit event are atomic.
        let mut conn = self.conn.lock().expect("orders store mutex poisoned");
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let sub: i64 = tx
            .query_row(
                "SELECT subtotal_minor FROM orders WHERE id=?",
                params![oid],
                |r| r.get(0),
            )
            .optional()?
            .ok_or_else(|| OrdersError::NotFound("order".into()))?;
        if d > sub {
            return Err(OrdersError::Invalid("discount exceeds subtotal".into()));
        }
        tx.execute(
            "UPDATE orders SET discount_minor=?,total_minor=?,updated_at=? WHERE id=?",
            params![d, sub - d, now, oid],
        )?;
        event_on(
            &tx,
            oid,
            None,
            actor,
            "DISCOUNT_SET",
            Some(reason),
            &serde_json::json!({"discount_minor": d}).to_string(),
            now,
        )?;
        tx.commit()?;
        get_order_on(&conn, oid)
    }

    pub fn transition(
        &self,
        oid: &str,
        to: OrderStatus,
        actor: &str,
        reason: Option<&str>,
        now: i64,
    ) -> Result<Order, OrdersError> {
        if to == OrderStatus::Cancelled && reason.map(str::trim).unwrap_or("").is_empty() {
            return Err(OrdersError::Invalid(
                "cancellation reason is required".into(),
            ));
        }
        // Status update + audit event are atomic.
        let mut conn = self.conn.lock().expect("orders store mutex poisoned");
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let from: String = tx
            .query_row("SELECT status FROM orders WHERE id=?", params![oid], |r| {
                r.get(0)
            })
            .optional()?
            .ok_or_else(|| OrdersError::NotFound("order".into()))?;
        let ok = match from.as_str() {
            "DRAFT" => matches!(to, OrderStatus::Submitted | OrderStatus::Cancelled),
            "SUBMITTED" => matches!(to, OrderStatus::Preparing | OrderStatus::Cancelled),
            "PREPARING" => matches!(to, OrderStatus::Ready | OrderStatus::Cancelled),
            "READY" => matches!(to, OrderStatus::Served),
            "SERVED" => matches!(to, OrderStatus::Completed),
            // COMPLETED and CANCELLED are terminal.
            _ => false,
        };
        if !ok {
            return Err(OrdersError::InvalidTransition {
                from,
                to: to.s().into(),
            });
        }
        tx.execute(
            "UPDATE orders SET status=?,updated_at=? WHERE id=?",
            params![to.s(), now, oid],
        )?;
        event_on(
            &tx,
            oid,
            None,
            actor,
            &format!("STATUS_{}", to.s()),
            reason.map(str::trim),
            "{}",
            now,
        )?;
        tx.commit()?;
        get_order_on(&conn, oid)
    }

    pub fn get_order(&self, oid: &str) -> Result<Order, OrdersError> {
        let conn = self.conn.lock().expect("orders store mutex poisoned");
        get_order_on(&conn, oid)
    }

    pub fn list_orders(&self, limit: usize) -> Result<Vec<Order>, OrdersError> {
        let lim = limit.clamp(1, 200) as i64;
        let conn = self.conn.lock().expect("orders store mutex poisoned");
        let mut s = conn.prepare(
            "SELECT id,order_number,order_type,table_id,customer_name,customer_phone,waiter_id,status,subtotal_minor,discount_minor,total_minor,currency,notes,created_at,updated_at FROM orders ORDER BY order_number DESC LIMIT ?",
        )?;
        let result = s
            .query_map(params![lim], row_order)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(result)
    }

    pub fn list_items(&self, oid: &str) -> Result<Vec<OrderItem>, OrdersError> {
        let conn = self.conn.lock().expect("orders store mutex poisoned");
        let mut s = conn.prepare(
            "SELECT id,order_id,menu_item_id,name_snapshot,unit_price_minor,quantity,total_minor,station,status,notes,created_at,updated_at FROM order_items WHERE order_id=? ORDER BY created_at,id",
        )?;
        let result = s
            .query_map(params![oid], row_item)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(result)
    }

    pub fn list_events(&self, oid: &str) -> Result<Vec<OrderEvent>, OrdersError> {
        let conn = self.conn.lock().expect("orders store mutex poisoned");
        let mut s = conn.prepare(
            "SELECT id,order_id,item_id,actor_id,event_type,reason,metadata_json,created_at FROM order_events WHERE order_id=? ORDER BY created_at,id",
        )?;
        let result = s
            .query_map(params![oid], |r| {
                Ok(OrderEvent {
                    id: r.get(0)?,
                    order_id: r.get(1)?,
                    item_id: r.get(2)?,
                    actor_id: r.get(3)?,
                    event_type: r.get(4)?,
                    reason: r.get(5)?,
                    metadata: serde_json::from_str(&r.get::<_, String>(6)?)
                        .unwrap_or(serde_json::Value::Null),
                    created_at: r.get(7)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(result)
    }
}

/// Fetch an order on an already-held connection (avoids re-locking the
/// store mutex from inside a method that already holds it).
fn get_order_on(conn: &Connection, oid: &str) -> Result<Order, OrdersError> {
    conn.query_row(
        "SELECT id,order_number,order_type,table_id,customer_name,customer_phone,waiter_id,status,subtotal_minor,discount_minor,total_minor,currency,notes,created_at,updated_at FROM orders WHERE id=?",
        params![oid],
        row_order,
    )
    .optional()?
    .ok_or_else(|| OrdersError::NotFound("order".into()))
}

/// Recompute order totals from active (non-cancelled) items, keeping the
/// invariant `total = subtotal - discount >= 0`.
fn recalc_on(conn: &Connection, oid: &str, now: i64) -> Result<(), OrdersError> {
    let sub: i64 = conn.query_row(
        "SELECT COALESCE(SUM(total_minor),0) FROM order_items WHERE order_id=? AND status<>'CANCELLED'",
        params![oid],
        |r| r.get(0),
    )?;
    let d: i64 = conn.query_row(
        "SELECT discount_minor FROM orders WHERE id=?",
        params![oid],
        |r| r.get(0),
    )?;
    if d > sub {
        return Err(OrdersError::Invalid("discount exceeds subtotal".into()));
    }
    conn.execute(
        "UPDATE orders SET subtotal_minor=?,total_minor=?,updated_at=? WHERE id=?",
        params![sub, sub - d, now, oid],
    )?;
    Ok(())
}

/// Append an audit event; metadata must be valid JSON. Always called inside
/// the caller's transaction so the business mutation and its audit event
/// commit or roll back together.
fn event_on(
    conn: &Connection,
    oid: &str,
    iid: Option<&str>,
    actor: &str,
    kind: &str,
    reason: Option<&str>,
    meta: &str,
    now: i64,
) -> Result<(), OrdersError> {
    serde_json::from_str::<serde_json::Value>(meta)
        .map_err(|_| OrdersError::Invalid("invalid event metadata".into()))?;
    conn.execute(
        "INSERT INTO order_events(id,order_id,item_id,actor_id,event_type,reason,metadata_json,created_at) VALUES(?,?,?,?,?,?,?,?)",
        params![Uuid::new_v4().to_string(), oid, iid, actor, kind, reason, meta, now],
    )?;
    Ok(())
}

fn row_order(r: &rusqlite::Row<'_>) -> rusqlite::Result<Order> {
    Ok(Order {
        id: r.get(0)?,
        order_number: r.get(1)?,
        order_type: OrderType::from_db(&r.get::<_, String>(2)?).map_err(corrupt_err)?,
        table_id: r.get(3)?,
        customer_name: r.get(4)?,
        customer_phone: r.get(5)?,
        waiter_id: r.get(6)?,
        status: OrderStatus::from_db(&r.get::<_, String>(7)?).map_err(corrupt_err)?,
        subtotal_minor: r.get(8)?,
        discount_minor: r.get(9)?,
        total_minor: r.get(10)?,
        currency: r.get(11)?,
        notes: r.get(12)?,
        created_at: r.get(13)?,
        updated_at: r.get(14)?,
    })
}

fn row_item(r: &rusqlite::Row<'_>) -> rusqlite::Result<OrderItem> {
    Ok(OrderItem {
        id: r.get(0)?,
        order_id: r.get(1)?,
        menu_item_id: r.get(2)?,
        name_snapshot: r.get(3)?,
        unit_price_minor: r.get(4)?,
        quantity: r.get(5)?,
        total_minor: r.get(6)?,
        station: r.get(7)?,
        status: ItemStatus::from_db(&r.get::<_, String>(8)?).map_err(corrupt_err)?,
        notes: r.get(9)?,
        created_at: r.get(10)?,
        updated_at: r.get(11)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> OrdersStore {
        OrdersStore::open(":memory:").unwrap()
    }

    /// Seed a store with one category, a menu item available for the given
    /// channels, and a table. Returns (category, menu item, table).
    fn seed(s: &OrdersStore, di: bool, to: bool, de: bool) -> (Category, MenuItem, Table) {
        let c = s.create_category("Food", 0, 1).unwrap();
        let m = s
            .create_menu_item(
                &c.id, "Chapati", None, "CHAPATI", 5000, "UGX", di, to, de, "ceo", 1,
            )
            .unwrap();
        let t = s.create_table("T1", 1).unwrap();
        (c, m, t)
    }

    fn dine_in_order(s: &OrdersStore, t: &Table) -> Order {
        s.create_order(
            OrderType::DineIn,
            Some(&t.id),
            None,
            None,
            "waiter",
            None,
            "UGX",
            1,
        )
        .unwrap()
    }

    #[test]
    fn order_lifecycle_and_money() {
        let s = store();
        let now = 1;
        let (_, m, t) = seed(&s, true, true, false);
        let o = dine_in_order(&s, &t);
        let i = s.add_item(&o.id, &m.id, 2, None, "waiter", now).unwrap();
        assert_eq!(i.total_minor, 10000);
        let o = s.set_discount(&o.id, 1000, "ceo", "approved", now).unwrap();
        assert_eq!((o.subtotal_minor, o.total_minor), (10000, 9000));
        let o = s
            .transition(&o.id, OrderStatus::Submitted, "waiter", None, now)
            .unwrap();
        let o = s
            .transition(&o.id, OrderStatus::Preparing, "cook", None, now)
            .unwrap();
        assert_eq!(o.status, OrderStatus::Preparing)
    }

    #[test]
    fn full_happy_path_lifecycle() {
        let s = store();
        let (_, m, t) = seed(&s, true, true, false);
        let o = dine_in_order(&s, &t);
        s.add_item(&o.id, &m.id, 1, None, "w", 1).unwrap();
        let o = s
            .transition(&o.id, OrderStatus::Submitted, "w", None, 2)
            .unwrap();
        assert_eq!(o.status, OrderStatus::Submitted);
        let o = s
            .transition(&o.id, OrderStatus::Preparing, "w", None, 3)
            .unwrap();
        assert_eq!(o.status, OrderStatus::Preparing);
        let o = s
            .transition(&o.id, OrderStatus::Ready, "w", None, 4)
            .unwrap();
        assert_eq!(o.status, OrderStatus::Ready);
        let o = s
            .transition(&o.id, OrderStatus::Served, "w", None, 5)
            .unwrap();
        assert_eq!(o.status, OrderStatus::Served);
        let o = s
            .transition(&o.id, OrderStatus::Completed, "w", None, 6)
            .unwrap();
        assert_eq!(o.status, OrderStatus::Completed);
        // Terminal states cannot move back into active states.
        assert!(matches!(
            s.transition(&o.id, OrderStatus::Preparing, "w", None, 7),
            Err(OrdersError::InvalidTransition { .. })
        ));
        assert!(matches!(
            s.transition(&o.id, OrderStatus::Cancelled, "w", Some("late"), 8),
            Err(OrdersError::InvalidTransition { .. })
        ));
    }

    #[test]
    fn invalid_transitions_are_rejected() {
        let s = store();
        let (_, m, t) = seed(&s, true, true, false);
        let o = dine_in_order(&s, &t);
        s.add_item(&o.id, &m.id, 1, None, "w", 1).unwrap();
        // Skip-ahead transition is not allowed.
        assert!(matches!(
            s.transition(&o.id, OrderStatus::Preparing, "w", None, 2),
            Err(OrdersError::InvalidTransition { .. })
        ));
        s.transition(&o.id, OrderStatus::Submitted, "w", None, 2)
            .unwrap();
        s.transition(&o.id, OrderStatus::Preparing, "w", None, 3)
            .unwrap();
        s.transition(&o.id, OrderStatus::Ready, "w", None, 4)
            .unwrap();
        // READY -> PREPARING (backwards) must fail.
        assert!(matches!(
            s.transition(&o.id, OrderStatus::Preparing, "w", None, 5),
            Err(OrdersError::InvalidTransition { .. })
        ));
        // Cancel then try to resurrect.
        let o2 = dine_in_order(&s, &t);
        s.transition(
            &o2.id,
            OrderStatus::Cancelled,
            "w",
            Some("customer left"),
            2,
        )
        .unwrap();
        assert!(matches!(
            s.transition(&o2.id, OrderStatus::Preparing, "w", None, 3),
            Err(OrdersError::InvalidTransition { .. })
        ));
    }

    #[test]
    fn cancellation_requires_reason() {
        let s = store();
        let (_, _, t) = seed(&s, true, true, false);
        let o = dine_in_order(&s, &t);
        assert!(matches!(
            s.transition(&o.id, OrderStatus::Cancelled, "w", None, 2),
            Err(OrdersError::Invalid(_))
        ));
        assert!(matches!(
            s.transition(&o.id, OrderStatus::Cancelled, "w", Some("   "), 2),
            Err(OrdersError::Invalid(_))
        ));
        let o = s
            .transition(&o.id, OrderStatus::Cancelled, "w", Some("customer left"), 2)
            .unwrap();
        assert_eq!(o.status, OrderStatus::Cancelled);
    }

    #[test]
    fn cancellation_preserves_history() {
        let s = store();
        let c = s.create_category("Snacks", 0, 1).unwrap();
        let m = s
            .create_menu_item(
                &c.id, "Donut", None, "DISPLAY", 3000, "UGX", true, true, false, "ceo", 1,
            )
            .unwrap();
        let o = s
            .create_order(OrderType::Takeaway, None, None, None, "w", None, "UGX", 1)
            .unwrap();
        let i = s.add_item(&o.id, &m.id, 1, None, "w", 1).unwrap();
        s.cancel_item(&i.id, "w", "customer cancelled", 2).unwrap();
        assert_eq!(s.get_order(&o.id).unwrap().total_minor, 0);
        assert_eq!(s.list_events(&o.id).unwrap().len(), 3)
    }

    #[test]
    fn cancelled_items_stay_in_history_and_out_of_totals() {
        let s = store();
        let (_, m, t) = seed(&s, true, true, false);
        let o = dine_in_order(&s, &t);
        let i1 = s.add_item(&o.id, &m.id, 2, None, "w", 1).unwrap();
        let i2 = s.add_item(&o.id, &m.id, 1, None, "w", 1).unwrap();
        let o = s.cancel_item(&i1.id, "w", "wrong item", 2).unwrap();
        // Totals exclude the cancelled line, but history retains it.
        assert_eq!((o.subtotal_minor, o.total_minor), (5000, 5000));
        let items = s.list_items(&o.id).unwrap();
        assert_eq!(items.len(), 2);
        let cancelled = items.iter().find(|i| i.id == i1.id).unwrap();
        assert_eq!(cancelled.status, ItemStatus::Cancelled);
        // Snapshot survives: name, unit price, quantity, line total intact.
        assert_eq!(cancelled.name_snapshot, "Chapati");
        assert_eq!(cancelled.unit_price_minor, 5000);
        assert_eq!(cancelled.quantity, 2);
        assert_eq!(cancelled.total_minor, 10000);
        // Audit event was written with the reason.
        let events = s.list_events(&o.id).unwrap();
        assert!(events.iter().any(|e| e.event_type == "ITEM_CANCELLED"
            && e.item_id.as_deref() == Some(i1.id.as_str())
            && e.reason.as_deref() == Some("wrong item")));
        // Cannot cancel twice.
        assert!(matches!(
            s.cancel_item(&i1.id, "w", "again", 3),
            Err(OrdersError::Invalid(_))
        ));
        // The remaining line still counts.
        assert!(items
            .iter()
            .any(|i| i.id == i2.id && i.status == ItemStatus::Ordered));
    }

    #[test]
    fn cancel_item_requires_reason() {
        let s = store();
        let (_, m, t) = seed(&s, true, true, false);
        let o = dine_in_order(&s, &t);
        let i = s.add_item(&o.id, &m.id, 1, None, "w", 1).unwrap();
        assert!(matches!(
            s.cancel_item(&i.id, "w", "  ", 2),
            Err(OrdersError::Invalid(_))
        ));
    }

    #[test]
    fn discount_rules() {
        let s = store();
        let (_, m, t) = seed(&s, true, true, false);
        let o = dine_in_order(&s, &t);
        s.add_item(&o.id, &m.id, 2, None, "w", 1).unwrap();
        // Negative discount rejected.
        assert!(matches!(
            s.set_discount(&o.id, -1, "ceo", "bad", 2),
            Err(OrdersError::Invalid(_))
        ));
        // Discount greater than subtotal rejected.
        assert!(matches!(
            s.set_discount(&o.id, 10001, "ceo", "too much", 2),
            Err(OrdersError::Invalid(_))
        ));
        // Missing reason rejected.
        assert!(matches!(
            s.set_discount(&o.id, 100, "ceo", " ", 2),
            Err(OrdersError::Invalid(_))
        ));
        // Discount equal to subtotal allowed -> total 0.
        let o = s.set_discount(&o.id, 10000, "ceo", "comp", 2).unwrap();
        assert_eq!(
            (o.subtotal_minor, o.discount_minor, o.total_minor),
            (10000, 10000, 0)
        );
    }

    #[test]
    fn discount_survives_item_cancellation_when_valid() {
        let s = store();
        let (_, m, t) = seed(&s, true, true, false);
        let o = dine_in_order(&s, &t);
        let i1 = s.add_item(&o.id, &m.id, 1, None, "w", 1).unwrap();
        s.add_item(&o.id, &m.id, 1, None, "w", 1).unwrap();
        s.set_discount(&o.id, 4000, "ceo", "loyal customer", 2)
            .unwrap();
        // After cancelling one line, subtotal (5000) still covers discount.
        let o = s.cancel_item(&i1.id, "w", "mistake", 3).unwrap();
        assert_eq!(
            (o.subtotal_minor, o.discount_minor, o.total_minor),
            (5000, 4000, 1000)
        );
    }

    #[test]
    fn money_overflow_is_rejected() {
        let s = store();
        let c = s.create_category("Food", 0, 1).unwrap();
        let m = s
            .create_menu_item(
                &c.id,
                "Expensive",
                None,
                "KITCHEN",
                i64::MAX,
                "UGX",
                true,
                true,
                true,
                "ceo",
                1,
            )
            .unwrap();
        let t = s.create_table("T1", 1).unwrap();
        let o = dine_in_order(&s, &t);
        assert!(matches!(
            s.add_item(&o.id, &m.id, 2, None, "w", 1),
            Err(OrdersError::Invalid(_))
        ));
        // Negative price is rejected at creation.
        assert!(s
            .create_menu_item(&c.id, "Bad", None, "KITCHEN", -1, "UGX", true, true, true, "ceo", 1)
            .is_err());
        // Zero/negative quantity rejected.
        assert!(s.add_item(&o.id, &m.id, 0, None, "w", 1).is_err());
        assert!(s.add_item(&o.id, &m.id, -3, None, "w", 1).is_err());
    }

    #[test]
    fn menu_availability_matches_order_type() {
        let s = store();
        // Item only available for dine-in.
        let (_, m, t) = seed(&s, true, false, false);
        let delivery_order = s
            .create_order(
                OrderType::Delivery,
                None,
                Some("Cust"),
                Some("0700123456"),
                "w",
                None,
                "UGX",
                1,
            )
            .unwrap();
        assert!(matches!(
            s.add_item(&delivery_order.id, &m.id, 1, None, "w", 1),
            Err(OrdersError::NotAvailable(_))
        ));
        let takeaway_order = s
            .create_order(OrderType::Takeaway, None, None, None, "w", None, "UGX", 1)
            .unwrap();
        assert!(matches!(
            s.add_item(&takeaway_order.id, &m.id, 1, None, "w", 1),
            Err(OrdersError::NotAvailable(_))
        ));
        // Dine-in order accepts the dine-in item.
        let dine = dine_in_order(&s, &t);
        assert!(s.add_item(&dine.id, &m.id, 1, None, "w", 1).is_ok());
    }

    #[test]
    fn currency_validation() {
        let s = store();
        let c = s.create_category("Food", 0, 1).unwrap();
        // Lowercase is normalized to uppercase.
        let m = s
            .create_menu_item(
                &c.id, "Tea", None, "BAR", 1000, "ugx", true, true, true, "ceo", 1,
            )
            .unwrap();
        assert_eq!(m.currency, "UGX");
        // Meaningless codes rejected.
        for bad in ["@@@", "123", "ab", "ABCD", "u x"] {
            assert!(
                s.create_menu_item(&c.id, "X", None, "BAR", 1000, bad, true, true, true, "ceo", 1)
                    .is_err(),
                "currency '{bad}' should be rejected"
            );
        }
        let t = s.create_table("T1", 1).unwrap();
        assert!(s
            .create_order(
                OrderType::DineIn,
                Some(&t.id),
                None,
                None,
                "w",
                None,
                "@@@",
                1
            )
            .is_err());
        let o = s
            .create_order(
                OrderType::DineIn,
                Some(&t.id),
                None,
                None,
                "w",
                None,
                "usd",
                1,
            )
            .unwrap();
        assert_eq!(o.currency, "USD");
    }

    #[test]
    fn blank_required_strings_rejected_and_trimmed() {
        let s = store();
        assert!(s.create_category("   ", 0, 1).is_err());
        assert!(s.create_table("", 1).is_err());
        let c = s.create_category(" Food ", 0, 1).unwrap();
        assert_eq!(c.name, "Food");
        assert!(s
            .create_menu_item(&c.id, " ", None, "BAR", 100, "UGX", true, true, true, "ceo", 1)
            .is_err());
        assert!(s
            .create_menu_item(&c.id, "Tea", None, " ", 100, "UGX", true, true, true, "ceo", 1)
            .is_err());
        let m = s
            .create_menu_item(
                &c.id,
                " Tea ",
                Some("  spiced  "),
                " BAR ",
                100,
                "UGX",
                true,
                true,
                true,
                "ceo",
                1,
            )
            .unwrap();
        assert_eq!(m.name, "Tea");
        assert_eq!(m.station, "BAR");
        assert_eq!(m.description.as_deref(), Some("spiced"));
    }

    #[test]
    fn duplicate_category_and_table_names_are_domain_errors() {
        let s = store();
        s.create_category("Food", 0, 1).unwrap();
        assert!(matches!(
            s.create_category("food", 0, 2),
            Err(OrdersError::Conflict(_))
        ));
        s.create_table("T1", 1).unwrap();
        assert!(matches!(
            s.create_table("T1", 2),
            Err(OrdersError::Conflict(_))
        ));
    }

    #[test]
    fn order_numbers_are_unique_and_increasing() {
        let s = store();
        let (_, _, t) = seed(&s, true, true, false);
        let a = dine_in_order(&s, &t);
        let b = dine_in_order(&s, &t);
        let c = dine_in_order(&s, &t);
        assert_eq!((a.order_number, b.order_number, c.order_number), (1, 2, 3));
        // The UUID, not the order number, is the unique identity.
        assert_ne!(a.id, b.id);
    }

    #[test]
    fn concurrent_order_creation_allocates_distinct_numbers() {
        let dir = std::env::temp_dir().join(format!("orders-core-test-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("orders.sqlite3");
        // Run schema migrations once up front: spawning two writers that both
        // open (and migrate) a brand-new database file concurrently is not a
        // supported pattern; production runs migrations once at app startup.
        drop(OrdersStore::open(&path).unwrap());
        let path2 = path.clone();
        let t1 = std::thread::spawn(move || {
            let s = OrdersStore::open(&path2).unwrap();
            let mut numbers = Vec::new();
            for i in 0..25 {
                let o = s
                    .create_order(OrderType::Takeaway, None, None, None, "w1", None, "UGX", i)
                    .unwrap();
                numbers.push(o.order_number);
            }
            numbers
        });
        let path3 = path.clone();
        let t2 = std::thread::spawn(move || {
            let s = OrdersStore::open(&path3).unwrap();
            let mut numbers = Vec::new();
            for i in 0..25 {
                let o = s
                    .create_order(OrderType::Takeaway, None, None, None, "w2", None, "UGX", i)
                    .unwrap();
                numbers.push(o.order_number);
            }
            numbers
        });
        let mut all = t1.join().unwrap();
        all.extend(t2.join().unwrap());
        all.sort_unstable();
        all.dedup();
        assert_eq!(all.len(), 50, "order numbers must be unique across writers");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn strict_enum_decoding_rejects_corrupt_values() {
        assert!(OrderStatus::from_db("GARBAGE").is_err());
        assert!(OrderType::from_db("").is_err());
        assert!(ItemStatus::from_db("ordered").is_err());
        assert_eq!(OrderType::from_db("DINE_IN").unwrap(), OrderType::DineIn);
        // Simulated database corruption surfaces as an error, not a
        // fallback to a valid state. (Writing a bad status directly is
        // impossible post-migration thanks to the CHECK constraint, so the
        // row-decoding layer is exercised with a literal bad value.)
        let s = store();
        let (_, _, t) = seed(&s, true, true, false);
        let o = dine_in_order(&s, &t);
        let conn = s.conn.lock().expect("orders store mutex poisoned");
        let row_err = (|| -> Result<Order, rusqlite::Error> {
            conn.query_row(
                "SELECT id,order_number,order_type,table_id,customer_name,customer_phone,waiter_id,'BOGUS',subtotal_minor,discount_minor,total_minor,currency,notes,created_at,updated_at FROM orders WHERE id=?",
                params![o.id],
                row_order,
            )
        })();
        assert!(row_err.is_err());
    }

    #[test]
    fn events_have_valid_json_metadata() {
        let s = store();
        let (_, m, t) = seed(&s, true, true, false);
        // Distinct timestamps: list_events orders by (created_at, id), and a
        // shared timestamp would make the event order nondeterministic.
        let o = s
            .create_order(
                OrderType::DineIn,
                Some(&t.id),
                None,
                None,
                "waiter",
                None,
                "UGX",
                10,
            )
            .unwrap();
        s.add_item(&o.id, &m.id, 3, None, "w", 11).unwrap();
        s.set_discount(&o.id, 100, "ceo", "promo", 12).unwrap();
        s.transition(&o.id, OrderStatus::Submitted, "w", None, 13)
            .unwrap();
        let events = s.list_events(&o.id).unwrap();
        let kinds: Vec<&str> = events.iter().map(|e| e.event_type.as_str()).collect();
        assert_eq!(
            kinds,
            vec![
                "ORDER_CREATED",
                "ITEM_ADDED",
                "DISCOUNT_SET",
                "STATUS_SUBMITTED"
            ]
        );
        for e in &events {
            assert!(
                e.metadata.is_object(),
                "{} metadata must be a JSON object",
                e.event_type
            );
        }
        // ITEM_ADDED carries the quantity in its metadata.
        let added = events
            .iter()
            .find(|e| e.event_type == "ITEM_ADDED")
            .unwrap();
        assert_eq!(added.metadata["quantity"], serde_json::json!(3));
        let disc = events
            .iter()
            .find(|e| e.event_type == "DISCOUNT_SET")
            .unwrap();
        assert_eq!(disc.metadata["discount_minor"], serde_json::json!(100));
    }

    #[test]
    fn historical_item_prices_are_snapshotted() {
        let s = store();
        let (_, m, t) = seed(&s, true, true, false);
        let o = dine_in_order(&s, &t);
        let i = s.add_item(&o.id, &m.id, 2, None, "w", 1).unwrap();
        // A later menu price change (simulated via direct SQL, as no
        // price-update API exists yet) must not touch the order line.
        s.conn
            .lock()
            .expect("orders store mutex poisoned")
            .execute(
                "UPDATE menu_items SET current_price_minor=99999 WHERE id=?",
                params![m.id],
            )
            .unwrap();
        let items = s.list_items(&o.id).unwrap();
        assert_eq!(items[0].unit_price_minor, 5000);
        assert_eq!(items[0].total_minor, 10000);
        assert_eq!(i.unit_price_minor, 5000);
    }

    #[test]
    fn dine_in_requires_table_and_table_must_exist() {
        let s = store();
        let (_, _, t) = seed(&s, true, true, false);
        assert!(matches!(
            s.create_order(OrderType::DineIn, None, None, None, "w", None, "UGX", 1),
            Err(OrdersError::Invalid(_))
        ));
        assert!(matches!(
            s.create_order(
                OrderType::DineIn,
                Some("no-such-table"),
                None,
                None,
                "w",
                None,
                "UGX",
                1
            ),
            Err(OrdersError::NotFound(_))
        ));
        // Table id is accepted when it exists.
        assert!(s
            .create_order(
                OrderType::DineIn,
                Some(&t.id),
                None,
                None,
                "w",
                None,
                "UGX",
                1
            )
            .is_ok());
    }

    #[test]
    fn items_locked_after_submission_states() {
        let s = store();
        let (_, m, t) = seed(&s, true, true, false);
        let o = dine_in_order(&s, &t);
        s.add_item(&o.id, &m.id, 1, None, "w", 1).unwrap();
        s.transition(&o.id, OrderStatus::Submitted, "w", None, 2)
            .unwrap();
        // SUBMITTED still allows edits (existing business rule).
        assert!(s.add_item(&o.id, &m.id, 1, None, "w", 2).is_ok());
        s.transition(&o.id, OrderStatus::Preparing, "w", None, 3)
            .unwrap();
        // PREPARING and beyond are locked.
        assert!(matches!(
            s.add_item(&o.id, &m.id, 1, None, "w", 3),
            Err(OrdersError::Invalid(_))
        ));
    }
}
