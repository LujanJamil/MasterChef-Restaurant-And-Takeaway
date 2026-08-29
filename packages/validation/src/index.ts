import { z } from "zod";

export const usernameSchema = z.string().trim().min(3).max(64).regex(/^[a-zA-Z0-9._-]+$/);
export const passwordSchema = z.string().min(8).max(128);
export const workerIdentitySchema = z.object({
  username: usernameSchema,
  displayName: z.string().trim().min(2).max(120),
  role: z.enum(["CEO", "MANAGER", "WAITER", "COOK", "CASHIER", "STOREKEEPER", "DELIVERY_RIDER"]),
});
