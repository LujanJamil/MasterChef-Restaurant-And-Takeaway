export type MenuCategoryKind = "FOOD" | "SNACKS" | "DRINKS" | "ICE_CREAM" | "CHAPATI" | "BREAD" | "OTHER";
export type OrderType = "DINE_IN" | "TAKEAWAY" | "DELIVERY";
export type OrderStatus = "OPEN" | "SUBMITTED" | "IN_PREPARATION" | "READY" | "SERVED" | "PACKED" | "COMPLETED" | "CANCELLED";
export type OrderItemStatus = "PENDING" | "SENT_TO_KITCHEN" | "IN_PREPARATION" | "READY" | "SERVED" | "CANCELLED";
export type PreparationStation = "KITCHEN" | "DISPLAY" | "FRIDGE" | "ICE_CREAM" | "CHAPATI" | "LOCAL_FOOD" | "OTHER";
export type PriceStatus = "ACTIVE" | "SCHEDULED" | "EXPIRED" | "CANCELLED";
export type CancellationReason = "CUSTOMER_REQUEST" | "OUT_OF_STOCK" | "WRONG_ITEM" | "DUPLICATE" | "MANAGEMENT" | "OTHER";

export interface MenuCategory {
  id: string; name: string; kind: MenuCategoryKind; sortOrder: number; active: boolean;
  createdBy: string; createdAt: number; updatedAt: number;
}

export interface MenuItem {
  id: string; categoryId: string; name: string; description: string | null;
  sku: string; station: PreparationStation; active: boolean; available: boolean;
  allowTakeaway: boolean; allowDineIn: boolean; allowDelivery: boolean;
  createdBy: string; createdAt: number; updatedAt: number;
}

export interface MenuPrice {
  id: string; menuItemId: string; amountMinor: number; currency: string;
  effectiveFrom: number; effectiveTo: number | null; status: PriceStatus; changedBy: string; createdAt: number;
}

export interface TableReference { id: string; label: string; active: boolean; }

export interface Order {
  id: string; orderNumber: string; type: OrderType; status: OrderStatus;
  tableId: string | null; customerName: string | null; customerPhone: string | null;
  waiterId: string | null; createdBy: string; deviceId: string;
  notes: string | null; subtotalMinor: number; discountMinor: number; totalMinor: number;
  currency: string; version: number; createdAt: number; updatedAt: number;
}

export interface OrderItem {
  id: string; orderId: string; menuItemId: string; itemNameSnapshot: string;
  unitPriceMinor: number; quantity: number; discountMinor: number; lineTotalMinor: number;
  station: PreparationStation; status: OrderItemStatus; notes: string | null;
  createdBy: string; createdAt: number; updatedAt: number;
}

export interface OrderItemCancellation {
  id: string; orderItemId: string; quantity: number; reason: CancellationReason;
  note: string | null; authorizedBy: string | null; cancelledBy: string; createdAt: number;
}

export interface OrderEvent {
  id: string; orderId: string; orderItemId: string | null;
  type: "CREATED" | "ITEM_ADDED" | "ITEM_UPDATED" | "ITEM_CANCELLED" | "STATUS_CHANGED" | "DISCOUNT_APPLIED" | "NOTE_CHANGED";
  actorId: string; deviceId: string; occurredAt: number; payload: string;
}

export interface OrderTotals { subtotalMinor: number; discountMinor: number; totalMinor: number; currency: string; }

export function calculateLineTotal(unitPriceMinor: number, quantity: number, discountMinor = 0): number {
  if (!Number.isSafeInteger(unitPriceMinor) || unitPriceMinor < 0) throw new Error("Invalid unit price");
  if (!Number.isSafeInteger(quantity) || quantity <= 0) throw new Error("Invalid quantity");
  if (!Number.isSafeInteger(discountMinor) || discountMinor < 0) throw new Error("Invalid discount");
  const gross = unitPriceMinor * quantity;
  if (!Number.isSafeInteger(gross) || discountMinor > gross) throw new Error("Invalid line discount");
  return gross - discountMinor;
}

export function calculateOrderTotals(items: readonly Pick<OrderItem, "unitPriceMinor" | "quantity" | "discountMinor">[], orderDiscountMinor = 0, currency = "UGX"): OrderTotals {
  const subtotalMinor = items.reduce((sum, item) => sum + calculateLineTotal(item.unitPriceMinor, item.quantity, 0), 0);
  const itemDiscounts = items.reduce((sum, item) => sum + (item.discountMinor ?? 0), 0);
  const discountMinor = itemDiscounts + orderDiscountMinor;
  if (!Number.isSafeInteger(discountMinor) || discountMinor < 0 || discountMinor > subtotalMinor) throw new Error("Invalid order discount");
  return { subtotalMinor, discountMinor, totalMinor: subtotalMinor - discountMinor, currency };
}

export function canAddItem(order: Pick<Order, "status">): boolean {
  return order.status === "OPEN" || order.status === "SUBMITTED" || order.status === "IN_PREPARATION";
}

export function canCancelItem(item: Pick<OrderItem, "status" | "quantity">): boolean {
  return item.quantity > 0 && !["SERVED", "CANCELLED"].includes(item.status);
}
