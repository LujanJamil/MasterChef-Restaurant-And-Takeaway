export const APP_NAME = "MasterChef" as const;
export const RESTAURANT_NAME = "Master Restaurant with Master Chefs" as const;
export const LOCATION_NAME = "Najjera II Central Market" as const;
export type UserStatus = "ACTIVE" | "SUSPENDED" | "INACTIVE";
export type WorkerRole = "CEO" | "MANAGER" | "WAITER" | "COOK" | "CASHIER" | "STOREKEEPER" | "DELIVERY_RIDER";
export interface WorkerIdentity { id:string; username:string; displayName:string; role:WorkerRole; status:UserStatus; createdAt:string; }
export const DEFAULT_ROLES:readonly WorkerRole[]=["CEO","MANAGER","WAITER","COOK","CASHIER","STOREKEEPER","DELIVERY_RIDER"] as const;
export type AttendanceStatus="OPEN"|"CLOSED"|"ADJUSTED";
export type ShiftType="DAY"|"NIGHT"|"CUSTOM";
export type ShiftStatus="SCHEDULED"|"PRESENT"|"ABSENT"|"EXCUSED"|"CANCELLED"|"REPLACED"|"COMPLETED";
export type ShiftSwapStatus="REQUESTED"|"APPROVED"|"REJECTED"|"CANCELLED";
export type PayStatus="DRAFT"|"APPROVED"|"PAID"|"VOIDED";
export type PaymentMethod="CASH"|"MTN_MOMO"|"AIRTEL_MONEY"|"OTHER";
export interface AttendanceRecord{id:string;workerId:string;deviceId:string;clockInAt:number;clockOutAt:number|null;breakMinutes:number;status:AttendanceStatus;notes?:string|null;createdAt:number;updatedAt:number;}
export interface AvailabilityRule{id:string;workerId:string;dayOfWeek:number;availableFromMinute:number;availableToMinute:number;preferredShift:ShiftType|null;active:boolean;validFrom:number;validTo:number|null;createdAt:number;updatedAt:number;}
export interface Shift{id:string;workerId:string;role:WorkerRole|string;shiftType:ShiftType;startsAt:number;endsAt:number;location:string;status:ShiftStatus;createdBy:string;createdAt:number;updatedAt:number;}
export interface ShiftSwap{id:string;shiftId:string;fromWorkerId:string;toWorkerId:string;reason?:string|null;status:ShiftSwapStatus;requestedBy:string;approvedBy?:string|null;createdAt:number;updatedAt:number;}
export interface PayRate{id:string;workerId:string;role:WorkerRole|string;shiftType:ShiftType;rateMinor:number;currency:string;effectiveFrom:number;effectiveTo:number|null;createdBy:string;createdAt:number;}
export interface PayRecord{id:string;workerId:string;periodStart:number;periodEnd:number;hoursWorked:number;rateMinor:number;amountMinor:number;currency:string;status:PayStatus;createdBy:string;createdAt:number;updatedAt:number;}
export interface PayrollAdjustment{id:string;workerId:string;payRecordId:string|null;kind:string;amountMinor:number;reason:string;createdBy:string;createdAt:number;}
export interface WorkerAdvance{id:string;workerId:string;amountMinor:number;currency:string;reason:string;status:"OUTSTANDING"|"SETTLED"|"VOIDED";createdBy:string;createdAt:number;}
export interface WorkerPayment{id:string;workerId:string;amountMinor:number;currency:string;paymentMethod:PaymentMethod;reference:string|null;paidBy:string;createdAt:number;}
export interface PayrollSummary{grossMinor:number;adjustmentsMinor:number;advancesMinor:number;paidMinor:number;balanceMinor:number;}
export * from "./orders.js";
