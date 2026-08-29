export const APP_NAME = "MasterChef" as const;
export const RESTAURANT_NAME = "Master Restaurant with Master Chefs" as const;
export const LOCATION_NAME = "Najjera II Central Market" as const;

export type UserStatus = "ACTIVE" | "SUSPENDED" | "INACTIVE";
export type WorkerRole = "CEO" | "MANAGER" | "WAITER" | "COOK" | "CASHIER" | "STOREKEEPER" | "DELIVERY_RIDER";

export interface WorkerIdentity {
  id: string;
  username: string;
  displayName: string;
  role: WorkerRole;
  status: UserStatus;
  createdAt: string;
}

export const DEFAULT_ROLES: readonly WorkerRole[] = [
  "CEO", "MANAGER", "WAITER", "COOK", "CASHIER", "STOREKEEPER", "DELIVERY_RIDER"
] as const;

/** Phase 3 — Workforce */
export type AttendanceStatus = "OPEN" | "CLOSED" | "ADJUSTED";
export type PayStatus = "DRAFT" | "APPROVED" | "PAID";

export interface AttendanceRecord {
  id: string;
  workerId: string;
  deviceId: string;
  clockInAt: number;
  clockOutAt: number | null;
  breakMinutes: number;
  status: AttendanceStatus;
  notes?: string | null;
  createdAt: number;
  updatedAt: number;
}

export interface Shift {
  id: string;
  workerId: string;
  role: string;
  startsAt: number;
  endsAt: number;
  location: string;
  createdBy: string;
  createdAt: number;
  updatedAt: number;
}

export interface PayRecord {
  id: string;
  workerId: string;
  periodStart: number;
  periodEnd: number;
  hoursWorked: number;
  rateMinor: number;
  amountMinor: number;
  currency: string;
  status: PayStatus;
  createdBy: string;
  createdAt: number;
  updatedAt: number;
}
