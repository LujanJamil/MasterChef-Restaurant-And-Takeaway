import type { MenuItem, MenuPrice, Order, OrderEvent, OrderItem, OrderItemStatus, OrderStatus, OrderTotals, PreparationStation } from "@masterchef/domain";
import { addOrderItemSchema, cancelOrderItemSchema, changeOrderStatusSchema, createOrderSchema, orderDiscountSchema } from "@masterchef/validation";

export interface OrderStore {
  getOrder(id: string): Promise<Order | null>;
  getItems(orderId: string): Promise<OrderItem[]>;
  getMenuItem(id: string): Promise<MenuItem | null>;
  getActivePrice(menuItemId: string, at: number): Promise<MenuPrice | null>;
  saveOrder(order: Order): Promise<void>;
  saveItem(item: OrderItem): Promise<void>;
  saveEvent(event: OrderEvent): Promise<void>;
}

const transitions: Record<OrderStatus, readonly OrderStatus[]> = {
  OPEN: ["SUBMITTED", "CANCELLED"],
  SUBMITTED: ["IN_PREPARATION", "CANCELLED"],
  IN_PREPARATION: ["READY", "CANCELLED"],
  READY: ["SERVED", "PACKED", "CANCELLED"],
  SERVED: ["COMPLETED"],
  PACKED: ["COMPLETED"],
  COMPLETED: [],
  CANCELLED: [],
};

function assertTransition(from: OrderStatus, to: OrderStatus): void {
  if (!transitions[from].includes(to)) throw new Error(`Invalid order transition: ${from} -> ${to}`);
}

function now(): number { return Date.now(); }
function uuid(): string {
  if (typeof crypto !== "undefined" && "randomUUID" in crypto) return crypto.randomUUID();
  throw new Error("Secure UUID generation is unavailable");
}

export class OrderService {
  constructor(private readonly store: OrderStore) {}

  async create(input: unknown, actorId: string): Promise<Order> {
    const value = createOrderSchema.parse(input);
    const timestamp = now();
    const order: Order = {
      id: uuid(), orderNumber: `MC-${timestamp}-${uuid().slice(0, 8).toUpperCase()}`,
      type: value.type, status: "OPEN", tableId: value.tableId, customerName: value.customerName,
      customerPhone: value.customerPhone, waiterId: value.waiterId, createdBy: actorId, deviceId: value.deviceId,
      notes: value.notes, subtotalMinor: 0, discountMinor: 0, totalMinor: 0, currency: value.currency,
      version: 1, createdAt: timestamp, updatedAt: timestamp,
    };
    await this.store.saveOrder(order);
    await this.event(order, null, "CREATED", actorId, { type: order.type });
    return order;
  }

  async addItem(input: unknown, actorId: string): Promise<OrderItem> {
    const value = addOrderItemSchema.parse(input);
    const order = await this.store.getOrder(value.orderId);
    if (!order || !["OPEN", "SUBMITTED", "IN_PREPARATION"].includes(order.status)) throw new Error("Order cannot accept items in its current state");
    const menu = await this.store.getMenuItem(value.menuItemId);
    if (!menu || !menu.active || !menu.available) throw new Error("Menu item is unavailable");
    if (order.type === "DINE_IN" && !menu.allowDineIn) throw new Error("Item is unavailable for dine-in");
    if (order.type === "TAKEAWAY" && !menu.allowTakeaway) throw new Error("Item is unavailable for takeaway");
    if (order.type === "DELIVERY" && !menu.allowDelivery) throw new Error("Item is unavailable for delivery");
    const price = await this.store.getActivePrice(menu.id, now());
    if (!price) throw new Error("No active price exists for this menu item");
    const gross = price.amountMinor * value.quantity;
    if (!Number.isSafeInteger(gross) || value.discountMinor > gross) throw new Error("Invalid item amount");
    const item: OrderItem = {
      id: uuid(), orderId: order.id, menuItemId: menu.id, itemNameSnapshot: menu.name,
      unitPriceMinor: price.amountMinor, quantity: value.quantity, discountMinor: value.discountMinor,
      lineTotalMinor: gross - value.discountMinor, station: menu.station as PreparationStation,
      status: "PENDING" as OrderItemStatus, notes: value.notes, createdBy: actorId, createdAt: now(), updatedAt: now(),
    };
    await this.store.saveItem(item);
    await this.recalculate(order, actorId);
    await this.event(order, item.id, "ITEM_ADDED", actorId, { quantity: item.quantity, unitPriceMinor: item.unitPriceMinor });
    return item;
  }

  async cancelItem(input: unknown, actorId: string, authorizedBy: string | null = null): Promise<void> {
    const value = cancelOrderItemSchema.parse(input);
    const items = await this.store.getItems(value.orderItemId);
    const item = items.find(i => i.id === value.orderItemId);
    if (!item || item.status === "SERVED" || item.status === "CANCELLED" || value.quantity > item.quantity) throw new Error("Invalid cancellation");
    item.quantity -= value.quantity;
    item.lineTotalMinor = item.unitPriceMinor * item.quantity - Math.min(item.discountMinor, item.unitPriceMinor * item.quantity);
    if (item.quantity === 0) item.status = "CANCELLED";
    item.updatedAt = now();
    await this.store.saveItem(item);
    const order = await this.store.getOrder(item.orderId);
    if (!order) throw new Error("Order not found");
    await this.recalculate(order, actorId);
    await this.event(order, item.id, "ITEM_CANCELLED", actorId, { quantity: value.quantity, reason: value.reason, note: value.note, authorizedBy });
  }

  async changeStatus(input: unknown, actorId: string): Promise<void> {
    const value = changeOrderStatusSchema.parse(input);
    const order = await this.store.getOrder(value.orderId);
    if (!order) throw new Error("Order not found");
    assertTransition(order.status, value.status);
    order.status = value.status; order.version += 1; order.updatedAt = now();
    await this.store.saveOrder(order);
    await this.event(order, null, "STATUS_CHANGED", actorId, { status: value.status });
  }

  async applyDiscount(input: unknown, actorId: string): Promise<void> {
    const value = orderDiscountSchema.parse(input);
    const order = await this.store.getOrder(value.orderId);
    if (!order || ["COMPLETED", "CANCELLED"].includes(order.status)) throw new Error("Order cannot be discounted");
    const items = await this.store.getItems(order.id);
    const subtotal = items.reduce((s, i) => s + i.unitPriceMinor * i.quantity, 0);
    if (value.discountMinor > subtotal) throw new Error("Discount exceeds subtotal");
    order.discountMinor = value.discountMinor; order.totalMinor = subtotal - value.discountMinor;
    order.version += 1; order.updatedAt = now(); await this.store.saveOrder(order);
    await this.event(order, null, "DISCOUNT_APPLIED", actorId, { discountMinor: value.discountMinor, reason: value.reason });
  }

  private async recalculate(order: Order, actorId: string): Promise<OrderTotals> {
    const items = await this.store.getItems(order.id);
    const subtotalMinor = items.reduce((s, i) => s + i.unitPriceMinor * i.quantity, 0);
    const itemDiscounts = items.reduce((s, i) => s + i.discountMinor, 0);
    order.subtotalMinor = subtotalMinor; order.totalMinor = Math.max(0, subtotalMinor - itemDiscounts - order.discountMinor);
    order.version += 1; order.updatedAt = now(); await this.store.saveOrder(order);
    void actorId;
    return { subtotalMinor, discountMinor: itemDiscounts + order.discountMinor, totalMinor: order.totalMinor, currency: order.currency };
  }

  private async event(order: Order, orderItemId: string | null, type: OrderEvent["type"], actorId: string, payload: unknown): Promise<void> {
    await this.store.saveEvent({ id: uuid(), orderId: order.id, orderItemId, type, actorId, deviceId: order.deviceId, occurredAt: now(), payload: JSON.stringify(payload) });
  }
}
