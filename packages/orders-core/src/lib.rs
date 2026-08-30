use rusqlite::{ params, Connection, OptionalExtension };
use serde::{ Deserialize, Serialize };
use std::path::Path;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum OrdersError {
    #[error("database error: {0}")] Db(#[from] rusqlite::Error),
    #[error("invalid input: {0}")] Invalid(String),
    #[error("not found: {0}")] NotFound(String),
    #[error("invalid order transition from {from} to {to}")] InvalidTransition {
        from: String,
        to: String,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum OrderType {
    DineIn,
    Takeaway,
    Delivery,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum OrderStatus {
    Draft,
    Submitted,
    Preparing,
    Ready,
    Served,
    Completed,
    Cancelled,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
    pub id: String,
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
    pub name_snapshot: String,
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
pub struct OrdersStore {
    conn: Connection,
}
impl OrdersStore {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, OrdersError> {
        let c = Connection::open(path)?;
        let s = Self { conn: c };
        s.migrate()?;
        Ok(s)
    }
    fn migrate(&self) -> Result<(), OrdersError> {
        self.conn.execute_batch(
            "PRAGMA foreign_keys=ON; CREATE TABLE IF NOT EXISTS categories(id TEXT PRIMARY KEY,name TEXT NOT NULL,active INTEGER NOT NULL DEFAULT 1,sort_order INTEGER NOT NULL DEFAULT 0,created_at INTEGER NOT NULL,updated_at INTEGER NOT NULL); CREATE UNIQUE INDEX IF NOT EXISTS categories_name_ci ON categories(lower(name)); CREATE TABLE IF NOT EXISTS menu_items(id TEXT PRIMARY KEY,category_id TEXT NOT NULL REFERENCES categories(id),name TEXT NOT NULL,description TEXT,station TEXT NOT NULL,active INTEGER NOT NULL DEFAULT 1,dine_in INTEGER NOT NULL DEFAULT 1,takeaway INTEGER NOT NULL DEFAULT 1,delivery INTEGER NOT NULL DEFAULT 0,current_price_minor INTEGER NOT NULL CHECK(current_price_minor>=0),currency TEXT NOT NULL DEFAULT 'UGX',created_at INTEGER NOT NULL,updated_at INTEGER NOT NULL); CREATE INDEX IF NOT EXISTS menu_category_active ON menu_items(category_id,active); CREATE TABLE IF NOT EXISTS menu_prices(id TEXT PRIMARY KEY,menu_item_id TEXT NOT NULL REFERENCES menu_items(id),price_minor INTEGER NOT NULL CHECK(price_minor>=0),currency TEXT NOT NULL,effective_from INTEGER NOT NULL,effective_to INTEGER,created_by TEXT NOT NULL,created_at INTEGER NOT NULL); CREATE INDEX IF NOT EXISTS menu_price_effective ON menu_prices(menu_item_id,effective_from); CREATE TABLE IF NOT EXISTS restaurant_tables(id TEXT PRIMARY KEY,name TEXT NOT NULL UNIQUE,active INTEGER NOT NULL DEFAULT 1,created_at INTEGER NOT NULL,updated_at INTEGER NOT NULL); CREATE TABLE IF NOT EXISTS orders(id TEXT PRIMARY KEY,order_number INTEGER NOT NULL UNIQUE,order_type TEXT NOT NULL CHECK(order_type IN ('DINE_IN','TAKEAWAY','DELIVERY')),table_id TEXT REFERENCES restaurant_tables(id),customer_name TEXT,customer_phone TEXT,waiter_id TEXT NOT NULL,status TEXT NOT NULL,subtotal_minor INTEGER NOT NULL DEFAULT 0 CHECK(subtotal_minor>=0),discount_minor INTEGER NOT NULL DEFAULT 0 CHECK(discount_minor>=0),total_minor INTEGER NOT NULL DEFAULT 0 CHECK(total_minor>=0),currency TEXT NOT NULL DEFAULT 'UGX',notes TEXT,created_at INTEGER NOT NULL,updated_at INTEGER NOT NULL,CHECK(total_minor=subtotal_minor-discount_minor),CHECK(order_type<>'DINE_IN' OR table_id IS NOT NULL)); CREATE INDEX IF NOT EXISTS orders_status ON orders(status,updated_at); CREATE TABLE IF NOT EXISTS order_items(id TEXT PRIMARY KEY,order_id TEXT NOT NULL REFERENCES orders(id) ON DELETE CASCADE,menu_item_id TEXT NOT NULL REFERENCES menu_items(id),name_snapshot TEXT NOT NULL,unit_price_minor INTEGER NOT NULL CHECK(unit_price_minor>=0),quantity INTEGER NOT NULL CHECK(quantity>0),total_minor INTEGER NOT NULL CHECK(total_minor>=0),station TEXT NOT NULL,status TEXT NOT NULL,notes TEXT,created_at INTEGER NOT NULL,updated_at INTEGER NOT NULL,CHECK(total_minor=unit_price_minor*quantity)); CREATE INDEX IF NOT EXISTS order_items_order ON order_items(order_id); CREATE TABLE IF NOT EXISTS order_events(id TEXT PRIMARY KEY,order_id TEXT NOT NULL REFERENCES orders(id),item_id TEXT,actor_id TEXT NOT NULL,event_type TEXT NOT NULL,reason TEXT,metadata_json TEXT NOT NULL,created_at INTEGER NOT NULL); CREATE INDEX IF NOT EXISTS order_events_order ON order_events(order_id,created_at); CREATE TABLE IF NOT EXISTS order_number_sequence(id INTEGER PRIMARY KEY CHECK(id=1),next_number INTEGER NOT NULL); INSERT OR IGNORE INTO order_number_sequence(id,next_number) VALUES(1,1);"
        )?;
        Ok(())
    }
    pub fn create_category(
        &self,
        name: &str,
        sort: i64,
        now: i64
    ) -> Result<Category, OrdersError> {
        let n = name.trim();
        if n.is_empty() {
            return Err(OrdersError::Invalid("category name is required".into()));
        }
        let id = Uuid::new_v4().to_string();
        self.conn.execute(
            "INSERT INTO categories(id,name,sort_order,created_at,updated_at) VALUES(?,?,?,?,?)",
            params![id, n, sort, now, now]
        )?;
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
        let mut s = self.conn.prepare(
            "SELECT id,name,active,sort_order,created_at,updated_at FROM categories ORDER BY sort_order,name"
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
        now: i64
    ) -> Result<MenuItem, OrdersError> {
        if price < 0 || currency.len() != 3 || station.trim().is_empty() || name.trim().is_empty() {
            return Err(OrdersError::Invalid("invalid menu item fields".into()));
        }
        if
            self.conn
                .query_row("SELECT 1 FROM categories WHERE id=? AND active=1", params![cat], |r|
                    r.get::<_, i64>(0)
                )
                .optional()?
                .is_none()
        {
            return Err(OrdersError::NotFound("category".into()));
        }
        let id = Uuid::new_v4().to_string();
        self.conn.execute(
            "INSERT INTO menu_items(id,category_id,name,description,station,current_price_minor,currency,dine_in,takeaway,delivery,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?)",
            params![
                id,
                cat,
                name.trim(),
                desc,
                station,
                price,
                currency,
                di as i64,
                to as i64,
                de as i64,
                now,
                now
            ]
        )?;
        self.conn.execute(
            "INSERT INTO menu_prices(id,menu_item_id,price_minor,currency,effective_from,created_by,created_at) VALUES(?,?,?,?,?,?,?)",
            params![Uuid::new_v4().to_string(), id, price, currency, now, actor, now]
        )?;
        Ok(MenuItem {
            id,
            category_id: cat.into(),
            name: name.trim().into(),
            description: desc.map(str::to_string),
            station: station.into(),
            active: true,
            dine_in: di,
            takeaway: to,
            delivery: de,
            current_price_minor: price,
            currency: currency.into(),
            created_at: now,
            updated_at: now,
        })
    }
    pub fn list_menu(&self, cat: Option<&str>) -> Result<Vec<MenuItem>, OrdersError> {
        let mut s = self.conn.prepare(
            "SELECT id,category_id,name,description,station,active,dine_in,takeaway,delivery,current_price_minor,currency,created_at,updated_at FROM menu_items WHERE active=1 AND (?1 IS NULL OR category_id=?1) ORDER BY name"
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
        self.conn.execute(
            "INSERT INTO restaurant_tables(id,name,created_at,updated_at) VALUES(?,?,?,?)",
            params![id, n, now, now]
        )?;
        Ok(Table { id, name: n.into(), active: true, created_at: now, updated_at: now })
    }
    pub fn list_tables(&self) -> Result<Vec<Table>, OrdersError> {
        let mut s = self.conn.prepare(
            "SELECT id,name,active,created_at,updated_at FROM restaurant_tables WHERE active=1 ORDER BY name"
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
    pub fn create_order(
        &self,
        k: OrderType,
        table: Option<&str>,
        cn: Option<&str>,
        cp: Option<&str>,
        waiter: &str,
        notes: Option<&str>,
        currency: &str,
        now: i64
    ) -> Result<Order, OrdersError> {
        if waiter.is_empty() || currency.len() != 3 {
            return Err(OrdersError::Invalid("invalid order fields".into()));
        }
        if k == OrderType::DineIn && table.is_none() {
            return Err(OrdersError::Invalid("dine-in orders require a table".into()));
        }
        if let Some(t) = table {
            if
                self.conn
                    .query_row(
                        "SELECT 1 FROM restaurant_tables WHERE id=? AND active=1",
                        params![t],
                        |r| r.get::<_, i64>(0)
                    )
                    .optional()?
                    .is_none()
            {
                return Err(OrdersError::NotFound("table".into()));
            }
        }
        let next: i64 = self.conn.query_row(
            "SELECT next_number FROM order_number_sequence WHERE id=1",
            [],
            |r| r.get(0)
        )?;
        self.conn.execute(
            "UPDATE order_number_sequence SET next_number=next_number+1 WHERE id=1",
            []
        )?;
        let id = Uuid::new_v4().to_string();
        self.conn.execute(
            "INSERT INTO orders(id,order_number,order_type,table_id,customer_name,customer_phone,waiter_id,status,currency,notes,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?,?,?,?,?)",
            params![id, next, k.s(), table, cn, cp, waiter, "DRAFT", currency, notes, now, now]
        )?;
        self.event(&id, None, waiter, "ORDER_CREATED", None, "{}", now)?;
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
            currency: currency.into(),
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
        now: i64
    ) -> Result<OrderItem, OrdersError> {
        if q <= 0 {
            return Err(OrdersError::Invalid("quantity must be positive".into()));
        }
        let st: String = self.conn
            .query_row("SELECT status FROM orders WHERE id=?", params![oid], |r| r.get(0))
            .optional()?
            .ok_or_else(|| OrdersError::NotFound("order".into()))?;
        if st != "DRAFT" && st != "SUBMITTED" {
            return Err(OrdersError::Invalid("items cannot be changed in this state".into()));
        }
        let (name, price, station): (String, i64, String) = self.conn
            .query_row(
                "SELECT name,current_price_minor,station FROM menu_items WHERE id=? AND active=1",
                params![mid],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))
            )
            .optional()?
            .ok_or_else(|| OrdersError::NotFound("menu item".into()))?;
        let total = price
            .checked_mul(q)
            .ok_or_else(|| OrdersError::Invalid("line total overflow".into()))?;
        let id = Uuid::new_v4().to_string();
        self.conn.execute(
            "INSERT INTO order_items(id,order_id,menu_item_id,name_snapshot,unit_price_minor,quantity,total_minor,station,status,notes,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?,?,?,?,?)",
            params![id, oid, mid, name, price, q, total, station, "ORDERED", notes, now, now]
        )?;
        self.recalc(oid, now)?;
        self.event(
            oid,
            Some(&id),
            actor,
            "ITEM_ADDED",
            None,
            &format!(r#"{{"quantity":{q}}}"#),
            now
        )?;
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
        now: i64
    ) -> Result<(), OrdersError> {
        if reason.trim().is_empty() {
            return Err(OrdersError::Invalid("cancellation reason is required".into()));
        }
        let (oid, st): (String, String) = self.conn
            .query_row("SELECT order_id,status FROM order_items WHERE id=?", params![iid], |r|
                Ok((r.get(0)?, r.get(1)?))
            )
            .optional()?
            .ok_or_else(|| OrdersError::NotFound("order item".into()))?;
        if st == "CANCELLED" {
            return Err(OrdersError::Invalid("item already cancelled".into()));
        }
        self.conn.execute(
            "UPDATE order_items SET status='CANCELLED',updated_at=? WHERE id=?",
            params![now, iid]
        )?;
        self.recalc(&oid, now)?;
        self.event(&oid, Some(iid), actor, "ITEM_CANCELLED", Some(reason), "{}", now)
    }
    pub fn set_discount(
        &self,
        oid: &str,
        d: i64,
        actor: &str,
        reason: &str,
        now: i64
    ) -> Result<Order, OrdersError> {
        if d < 0 || reason.trim().is_empty() {
            return Err(OrdersError::Invalid("invalid discount".into()));
        }
        let sub: i64 = self.conn
            .query_row("SELECT subtotal_minor FROM orders WHERE id=?", params![oid], |r| r.get(0))
            .optional()?
            .ok_or_else(|| OrdersError::NotFound("order".into()))?;
        if d > sub {
            return Err(OrdersError::Invalid("discount exceeds subtotal".into()));
        }
        self.conn.execute(
            "UPDATE orders SET discount_minor=?,total_minor=?,updated_at=? WHERE id=?",
            params![d, sub - d, now, oid]
        )?;
        self.event(
            oid,
            None,
            actor,
            "DISCOUNT_SET",
            Some(reason),
            &format!(r#"{{"discount_minor":{d}}}"#),
            now
        )?;
        self.get_order(oid)
    }
    pub fn transition(
        &self,
        oid: &str,
        to: OrderStatus,
        actor: &str,
        reason: Option<&str>,
        now: i64
    ) -> Result<Order, OrdersError> {
        let from: String = self.conn
            .query_row("SELECT status FROM orders WHERE id=?", params![oid], |r| r.get(0))
            .optional()?
            .ok_or_else(|| OrdersError::NotFound("order".into()))?;
        let ok = match from.as_str() {
            "DRAFT" => matches!(to, OrderStatus::Submitted | OrderStatus::Cancelled),
            "SUBMITTED" => matches!(to, OrderStatus::Preparing | OrderStatus::Cancelled),
            "PREPARING" => matches!(to, OrderStatus::Ready | OrderStatus::Cancelled),
            "READY" => matches!(to, OrderStatus::Served),
            "SERVED" => matches!(to, OrderStatus::Completed),
            _ => false,
        };
        if !ok {
            return Err(OrdersError::InvalidTransition { from, to: to.s().into() });
        }
        if to == OrderStatus::Cancelled && reason.map(str::trim).unwrap_or("").is_empty() {
            return Err(OrdersError::Invalid("cancellation reason is required".into()));
        }
        self.conn.execute(
            "UPDATE orders SET status=?,updated_at=? WHERE id=?",
            params![to.s(), now, oid]
        )?;
        self.event(oid, None, actor, &format!("STATUS_{}", to.s()), reason, "{}", now)?;
        self.get_order(oid)
    }
    pub fn get_order(&self, oid: &str) -> Result<Order, OrdersError> {
        self.conn
            .query_row(
                "SELECT id,order_number,order_type,table_id,customer_name,customer_phone,waiter_id,status,subtotal_minor,discount_minor,total_minor,currency,notes,created_at,updated_at FROM orders WHERE id=?",
                params![oid],
                row_order
            )
            .optional()?
            .ok_or_else(|| OrdersError::NotFound("order".into()))
    }
    pub fn list_orders(&self, limit: usize) -> Result<Vec<Order>, OrdersError> {
        let lim = limit.clamp(1, 200) as i64;
        let mut s = self.conn.prepare(
            "SELECT id,order_number,order_type,table_id,customer_name,customer_phone,waiter_id,status,subtotal_minor,discount_minor,total_minor,currency,notes,created_at,updated_at FROM orders ORDER BY order_number DESC LIMIT ?"
        )?;
        let result = s
            .query_map(params![lim], row_order)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(result)
    }
    pub fn list_items(&self, oid: &str) -> Result<Vec<OrderItem>, OrdersError> {
        let mut s = self.conn.prepare(
            "SELECT id,order_id,menu_item_id,name_snapshot,unit_price_minor,quantity,total_minor,station,status,notes,created_at,updated_at FROM order_items WHERE order_id=? ORDER BY created_at,id"
        )?;
        let result = s
            .query_map(params![oid], row_item)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(result)
    }
    pub fn list_events(&self, oid: &str) -> Result<Vec<OrderEvent>, OrdersError> {
        let mut s = self.conn.prepare(
            "SELECT id,order_id,item_id,actor_id,event_type,reason,metadata_json,created_at FROM order_events WHERE order_id=? ORDER BY created_at,id"
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
    fn recalc(&self, oid: &str, now: i64) -> Result<(), OrdersError> {
        let sub: i64 = self.conn.query_row(
            "SELECT COALESCE(SUM(total_minor),0) FROM order_items WHERE order_id=? AND status<>'CANCELLED'",
            params![oid],
            |r| r.get(0)
        )?;
        let d: i64 = self.conn.query_row(
            "SELECT discount_minor FROM orders WHERE id=?",
            params![oid],
            |r| r.get(0)
        )?;
        if d > sub {
            return Err(OrdersError::Invalid("discount exceeds subtotal".into()));
        }
        self.conn.execute(
            "UPDATE orders SET subtotal_minor=?,total_minor=?,updated_at=? WHERE id=?",
            params![sub, sub - d, now, oid]
        )?;
        Ok(())
    }
    fn event(
        &self,
        oid: &str,
        iid: Option<&str>,
        actor: &str,
        kind: &str,
        reason: Option<&str>,
        meta: &str,
        now: i64
    ) -> Result<(), OrdersError> {
        serde_json
            ::from_str::<serde_json::Value>(meta)
            .map_err(|_| OrdersError::Invalid("invalid event metadata".into()))?;
        self.conn.execute(
            "INSERT INTO order_events(id,order_id,item_id,actor_id,event_type,reason,metadata_json,created_at) VALUES(?,?,?,?,?,?,?,?)",
            params![Uuid::new_v4().to_string(), oid, iid, actor, kind, reason, meta, now]
        )?;
        Ok(())
    }
}
fn row_order(r: &rusqlite::Row<'_>) -> rusqlite::Result<Order> {
    Ok(Order {
        id: r.get(0)?,
        order_number: r.get(1)?,
        order_type: match r.get::<_, String>(2)?.as_str() {
            "TAKEAWAY" => OrderType::Takeaway,
            "DELIVERY" => OrderType::Delivery,
            _ => OrderType::DineIn,
        },
        table_id: r.get(3)?,
        customer_name: r.get(4)?,
        customer_phone: r.get(5)?,
        waiter_id: r.get(6)?,
        status: match r.get::<_, String>(7)?.as_str() {
            "SUBMITTED" => OrderStatus::Submitted,
            "PREPARING" => OrderStatus::Preparing,
            "READY" => OrderStatus::Ready,
            "SERVED" => OrderStatus::Served,
            "COMPLETED" => OrderStatus::Completed,
            "CANCELLED" => OrderStatus::Cancelled,
            _ => OrderStatus::Draft,
        },
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
        status: match r.get::<_, String>(8)?.as_str() {
            "PREPARING" => ItemStatus::Preparing,
            "READY" => ItemStatus::Ready,
            "SERVED" => ItemStatus::Served,
            "CANCELLED" => ItemStatus::Cancelled,
            _ => ItemStatus::Ordered,
        },
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
    #[test]
    fn order_lifecycle_and_money() {
        let s = store();
        let now = 1;
        let c = s.create_category("Food", 0, now).unwrap();
        let m = s
            .create_menu_item(
                &c.id,
                "Chapati",
                None,
                "CHAPATI",
                5000,
                "UGX",
                true,
                true,
                false,
                "ceo",
                now
            )
            .unwrap();
        let t = s.create_table("T1", now).unwrap();
        let o = s
            .create_order(OrderType::DineIn, Some(&t.id), None, None, "waiter", None, "UGX", now)
            .unwrap();
        let i = s.add_item(&o.id, &m.id, 2, None, "waiter", now).unwrap();
        assert_eq!(i.total_minor, 10000);
        let o = s.set_discount(&o.id, 1000, "ceo", "approved", now).unwrap();
        assert_eq!((o.subtotal_minor, o.total_minor), (10000, 9000));
        let o = s.transition(&o.id, OrderStatus::Submitted, "waiter", None, now).unwrap();
        let o = s.transition(&o.id, OrderStatus::Preparing, "cook", None, now).unwrap();
        assert_eq!(o.status, OrderStatus::Preparing)
    }
    #[test]
    fn cancellation_preserves_history() {
        let s = store();
        let c = s.create_category("Snacks", 0, 1).unwrap();
        let m = s
            .create_menu_item(
                &c.id,
                "Donut",
                None,
                "DISPLAY",
                3000,
                "UGX",
                true,
                true,
                false,
                "ceo",
                1
            )
            .unwrap();
        let o = s.create_order(OrderType::Takeaway, None, None, None, "w", None, "UGX", 1).unwrap();
        let i = s.add_item(&o.id, &m.id, 1, None, "w", 1).unwrap();
        s.cancel_item(&i.id, "w", "customer cancelled", 2).unwrap();
        assert_eq!(s.get_order(&o.id).unwrap().total_minor, 0);
        assert_eq!(s.list_events(&o.id).unwrap().len(), 3)
    }
}
