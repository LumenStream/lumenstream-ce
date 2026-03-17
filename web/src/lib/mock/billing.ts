import type {
  CreateRechargeOrderRequest,
  Plan,
  PurchaseResult,
  RechargeOrder,
  Subscription,
  Wallet,
} from "@/lib/types/billing";

let idSeed = 5000;

function nextId(prefix: string): string {
  idSeed += 1;
  return `${prefix}-${idSeed}`;
}

const mockWallet: Wallet = {
  user_id: "user-admin-001",
  balance: "128.50",
  total_recharged: "200.00",
  total_spent: "71.50",
  updated_at: new Date().toISOString(),
};

const mockPlans: Plan[] = [
  {
    id: "plan-basic",
    code: "basic",
    name: "基础套餐",
    price: "15.00",
    duration_days: 30,
    traffic_quota_bytes: 107374182400, // 100GB
    traffic_window_days: 30,
    enabled: true,
    updated_at: new Date().toISOString(),
  },
  {
    id: "plan-standard",
    code: "standard",
    name: "标准套餐",
    price: "29.00",
    duration_days: 30,
    traffic_quota_bytes: 322122547200, // 300GB
    traffic_window_days: 30,
    enabled: true,
    updated_at: new Date().toISOString(),
  },
  {
    id: "plan-premium",
    code: "premium",
    name: "高级套餐",
    price: "59.00",
    duration_days: 30,
    traffic_quota_bytes: 858993459200, // 800GB
    traffic_window_days: 30,
    enabled: true,
    updated_at: new Date().toISOString(),
  },
  {
    id: "plan-annual",
    code: "annual",
    name: "年度会员",
    price: "299.00",
    duration_days: 365,
    traffic_quota_bytes: 536870912000, // 500GB
    traffic_window_days: 30,
    enabled: true,
    updated_at: new Date().toISOString(),
  },
];

const mockOrders: Map<string, RechargeOrder> = new Map();

export async function mockGetWallet(): Promise<Wallet> {
  return { ...mockWallet };
}

export async function mockGetPlans(): Promise<Plan[]> {
  return mockPlans.filter((plan) => plan.enabled).map((plan) => ({ ...plan }));
}

export async function mockCreateRechargeOrder(
  request: CreateRechargeOrderRequest
): Promise<RechargeOrder> {
  const now = new Date();
  const expiresAt = new Date(now.getTime() + 30 * 60 * 1000); // 30 minutes
  const order: RechargeOrder = {
    id: nextId("order"),
    user_id: mockWallet.user_id,
    out_trade_no: nextId("trade"),
    channel: request.channel || "alipay",
    amount: request.amount,
    status: "pending",
    subject: request.subject || "账户充值",
    provider_trade_no: null,
    paid_at: null,
    expires_at: expiresAt.toISOString(),
    created_at: now.toISOString(),
    updated_at: now.toISOString(),
  };

  mockOrders.set(order.id, order);

  // Simulate payment completion after 2 seconds in mock mode
  setTimeout(() => {
    const stored = mockOrders.get(order.id);
    if (stored && stored.status === "pending") {
      stored.status = "paid";
      stored.paid_at = new Date().toISOString();
      stored.updated_at = new Date().toISOString();
      const currentBalance = parseFloat(mockWallet.balance);
      const addedAmount = parseFloat(stored.amount);
      mockWallet.balance = (currentBalance + addedAmount).toFixed(2);
      mockWallet.total_recharged = (parseFloat(mockWallet.total_recharged) + addedAmount).toFixed(
        2
      );
      mockWallet.updated_at = new Date().toISOString();
    }
  }, 2000);

  return { ...order };
}

export async function mockGetRechargeOrder(orderId: string): Promise<RechargeOrder> {
  const order = mockOrders.get(orderId);
  if (!order) {
    throw { status: 404, message: "订单不存在" };
  }
  return { ...order };
}

export async function mockPurchasePlan(planId: string): Promise<PurchaseResult> {
  const plan = mockPlans.find((p) => p.id === planId);
  if (!plan) {
    throw { status: 404, message: "套餐不存在" };
  }

  const currentBalance = parseFloat(mockWallet.balance);
  const planPrice = parseFloat(plan.price);

  if (currentBalance < planPrice) {
    throw { status: 402, message: "余额不足，请先充值" };
  }

  const newBalance = (currentBalance - planPrice).toFixed(2);
  mockWallet.balance = newBalance;
  mockWallet.total_spent = (parseFloat(mockWallet.total_spent) + planPrice).toFixed(2);
  mockWallet.updated_at = new Date().toISOString();

  const now = new Date();
  const expiresAt = new Date(now.getTime() + plan.duration_days * 24 * 60 * 60 * 1000);

  const subscription: Subscription = {
    id: nextId("sub"),
    user_id: mockWallet.user_id,
    plan_id: plan.id,
    plan_code: plan.code,
    plan_name: plan.name,
    plan_price: plan.price,
    duration_days: plan.duration_days,
    traffic_quota_bytes: plan.traffic_quota_bytes,
    traffic_window_days: plan.traffic_window_days,
    status: "active",
    started_at: now.toISOString(),
    expires_at: expiresAt.toISOString(),
    replaced_at: null,
    updated_at: now.toISOString(),
  };

  return {
    wallet: { ...mockWallet },
    subscription,
    charged_amount: plan.price,
    proration: null,
  };
}
