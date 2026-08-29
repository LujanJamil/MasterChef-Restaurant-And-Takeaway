export type AuditAction =
  | "LOGIN" | "LOGIN_FAILED" | "LOGOUT" | "PERMISSION_DENIED"
  | "USER_CREATED" | "USER_UPDATED" | "ROLE_CHANGED" | "PERMISSION_CHANGED"
  | "CEO_OVERRIDE" | "SETTINGS_CHANGED";

export interface AuditEvent {
  id: string;
  occurredAt: string;
  actorUserId: string | null;
  deviceId: string;
  action: AuditAction;
  entityType?: string;
  entityId?: string;
  reason?: string;
  metadata?: Record<string, string | number | boolean | null>;
}

export function createAuditEvent(input: Omit<AuditEvent, "id">): AuditEvent {
  return { id: crypto.randomUUID(), ...input };
}
