import { describe, expect, it, vi } from "vitest";

import { getRechargeOrderWebSocketUrl } from "./billing";
import { getNotificationsWebSocketUrl } from "./notifications";

vi.mock("@/lib/api/client", () => ({
  apiRequest: vi.fn(),
  getApiBaseUrl: vi.fn(() => "https://api.example.com"),
}));

describe("realtime websocket urls", () => {
  it("builds notifications websocket url with token", () => {
    expect(getNotificationsWebSocketUrl("demo-token")).toBe(
      "wss://api.example.com/api/notifications/ws?token=demo-token"
    );
  });

  it("builds recharge-order websocket url with token", () => {
    expect(getRechargeOrderWebSocketUrl("order-001", "demo-token")).toBe(
      "wss://api.example.com/billing/recharge/orders/order-001/ws?token=demo-token"
    );
  });
});
