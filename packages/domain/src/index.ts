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
