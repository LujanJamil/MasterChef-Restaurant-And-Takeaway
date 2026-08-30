import { z } from "zod";

export const usernameSchema = z.string().trim().min(3).max(64).regex(/^[a-zA-Z0-9._-]+$/);
export const passwordSchema = z.string().min(8).max(128);
export const workerIdentitySchema = z.object({ username: usernameSchema, displayName: z.string().trim().min(2).max(120), role: z.enum(["CEO", "MANAGER", "WAITER", "COOK", "CASHIER", "STOREKEEPER", "DELIVERY_RIDER"]) });

const moneyMinor = z.number().int().nonnegative().max(Number.MAX_SAFE_INTEGER);
const id = z.string().uuid();
const name = z.string().trim().min(1).max(120);

export const menuCategorySchema = z.object({ name, kind: z.enum(["FOOD","SNACKS","DRINKS","ICE_CREAM","CHAPATI","BREAD","OTHER"]), sortOrder: z.number().int().nonnegative().max(100000), active: z.boolean() }).strict();
export const menuItemSchema = z.object({ categoryId: id, name, description: z.string().trim().max(1000).nullable(), sku: z.string().trim().min(1).max(64).regex(/^[A-Z0-9._-]+$/), station: z.enum(["KITCHEN","DISPLAY","FRIDGE","ICE_CREAM","CHAPATI","LOCAL_FOOD","OTHER"]), active: z.boolean(), available: z.boolean(), allowTakeaway: z.boolean(), allowDineIn: z.boolean(), allowDelivery: z.boolean() }).strict();
export const menuPriceSchema = z.object({ menuItemId: id, amountMinor: moneyMinor, currency: z.string().trim().length(3).toUpperCase(), effectiveFrom: z.number().int().positive(), effectiveTo: z.number().int().positive().nullable(), status: z.enum(["ACTIVE","SCHEDULED","EXPIRED","CANCELLED"]) }).strict().refine(v => v.effectiveTo === null || v.effectiveTo > v.effectiveFrom, { message: "effectiveTo must be after effectiveFrom" });
export const tableSchema = z.object({ label: name, active: z.boolean() }).strict();

export const createOrderSchema = z.object({ type: z.enum(["DINE_IN","TAKEAWAY","DELIVERY"]), tableId: id.nullable(), customerName: z.string().trim().max(120).nullable(), customerPhone: z.string().trim().max(32).nullable(), waiterId: id.nullable(), notes: z.string().trim().max(2000).nullable(), currency: z.string().trim().length(3).toUpperCase(), deviceId: id }).strict().superRefine((v, ctx) => { if (v.type === "DINE_IN" && !v.tableId) ctx.addIssue({ code: z.ZodIssueCode.custom, path: ["tableId"], message: "Dine-in orders require a table" }); if (v.type !== "DINE_IN" && v.tableId) ctx.addIssue({ code: z.ZodIssueCode.custom, path: ["tableId"], message: "Only dine-in orders may have a table" }); if (v.type === "DELIVERY" && !v.customerPhone) ctx.addIssue({ code: z.ZodIssueCode.custom, path: ["customerPhone"], message: "Delivery orders require a customer phone" }); });
export const addOrderItemSchema = z.object({ orderId: id, menuItemId: id, quantity: z.number().int().positive().max(10000), discountMinor: moneyMinor.default(0), notes: z.string().trim().max(1000).nullable() }).strict();
export const cancelOrderItemSchema = z.object({ orderItemId: id, quantity: z.number().int().positive().max(10000), reason: z.enum(["CUSTOMER_REQUEST","OUT_OF_STOCK","WRONG_ITEM","DUPLICATE","MANAGEMENT","OTHER"]), note: z.string().trim().max(1000).nullable() }).strict();
export const changeOrderStatusSchema = z.object({ orderId: id, status: z.enum(["OPEN","SUBMITTED","IN_PREPARATION","READY","SERVED","PACKED","COMPLETED","CANCELLED"]) }).strict();
export const orderDiscountSchema = z.object({ orderId: id, discountMinor: moneyMinor, reason: z.string().trim().min(1).max(500) }).strict();
