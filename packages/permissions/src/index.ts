import type { WorkerRole } from "@masterchef/domain";

export type Permission =
  | "system.manage" | "users.manage" | "workers.view" | "workers.manage"
  | "orders.view" | "orders.create" | "orders.edit" | "orders.cancel"
  | "payments.view" | "payments.receive" | "payments.refund"
  | "cash.open" | "cash.close" | "cash.reconcile"
  | "inventory.view" | "inventory.issue" | "inventory.adjust"
  | "attendance.view" | "attendance.manage" | "pay.view" | "pay.manage"
  | "reports.view" | "audit.view" | "delivery.manage";

export const ROLE_PERMISSIONS: Readonly<Record<WorkerRole, readonly Permission[]>> = {
  CEO: [
    "system.manage", "users.manage", "workers.view", "workers.manage", "orders.view", "orders.create", "orders.edit", "orders.cancel",
    "payments.view", "payments.receive", "payments.refund", "cash.open", "cash.close", "cash.reconcile",
    "inventory.view", "inventory.issue", "inventory.adjust", "attendance.view", "attendance.manage", "pay.view", "pay.manage",
    "reports.view", "audit.view", "delivery.manage"
  ],
  MANAGER: ["workers.view", "orders.view", "orders.create", "orders.edit", "payments.view", "payments.receive", "cash.open", "cash.close", "cash.reconcile", "inventory.view", "attendance.view", "attendance.manage", "reports.view", "delivery.manage"],
  WAITER: ["orders.view", "orders.create", "orders.edit", "payments.receive"],
  COOK: ["orders.view"],
  CASHIER: ["orders.view", "payments.view", "payments.receive", "payments.refund", "cash.open", "cash.close", "cash.reconcile"],
  STOREKEEPER: ["inventory.view", "inventory.issue"],
  DELIVERY_RIDER: ["orders.view", "delivery.manage"]
};

export interface PermissionContext {
  role: WorkerRole;
  directGrants?: readonly Permission[];
  directRevokes?: readonly Permission[];
}

export function hasPermission(context: PermissionContext, permission: Permission): boolean {
  if (context.role === "CEO") return true;
  if (context.directRevokes?.includes(permission)) return false;
  return ROLE_PERMISSIONS[context.role].includes(permission) || Boolean(context.directGrants?.includes(permission));
}
