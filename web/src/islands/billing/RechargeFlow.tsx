import { useEffect, useRef, useState } from "react";

import { Modal } from "@/components/domain/Modal";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { createRechargeOrder, getRechargeOrderWebSocketUrl } from "@/lib/api/billing";
import { getAccessToken } from "@/lib/auth/token";
import type { ApiError } from "@/lib/api/client";
import { toast } from "@/lib/notifications/toast-store";
import type { RechargeOrder } from "@/lib/types/billing";

const PRESET_AMOUNTS = [
  { label: "¥50", value: "50.00" },
  { label: "¥100", value: "100.00" },
  { label: "¥200", value: "200.00" },
  { label: "¥500", value: "500.00" },
];

interface RechargeFlowProps {
  open: boolean;
  onClose: () => void;
  onSuccess?: () => void;
}

interface RechargeOrderSocketEvent {
  event: string;
  order: RechargeOrder;
  emitted_at: string;
}

export function RechargeFlow({ open, onClose, onSuccess }: RechargeFlowProps) {
  const [selectedAmount, setSelectedAmount] = useState<string>("100.00");
  const [customAmount, setCustomAmount] = useState<string>("");
  const [order, setOrder] = useState<RechargeOrder | null>(null);
  const [loading, setLoading] = useState(false);
  const [polling, setPolling] = useState(false);
  const successHandledOrderIdRef = useRef<string | null>(null);
  const terminalToastKeyRef = useRef<string | null>(null);

  function getAmount(): string {
    if (customAmount) {
      const parsed = parseFloat(customAmount);
      if (!isNaN(parsed) && parsed > 0) {
        return parsed.toFixed(2);
      }
    }
    return selectedAmount;
  }

  async function handleCreateOrder() {
    const amount = getAmount();
    const amountNum = parseFloat(amount);
    if (amountNum < 1) {
      toast.error("充值金额不能小于 ¥1.00");
      return;
    }

    setLoading(true);

    try {
      const newOrder = await createRechargeOrder({ amount });
      setOrder(newOrder);
    } catch (cause) {
      const apiError = cause as ApiError;
      toast.error(apiError.message || "创建订单失败");
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    if (!open || !order || order.status !== "pending" || typeof WebSocket === "undefined") {
      return;
    }

    const token = getAccessToken();
    if (!token) {
      setPolling(false);
      return;
    }

    const orderId = order.id;
    let socket: WebSocket | null = null;
    let closed = false;
    let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
    let reconnectDelay = 1000;

    const connect = () => {
      if (closed) {
        return;
      }

      socket = new WebSocket(getRechargeOrderWebSocketUrl(orderId, token));
      setPolling(true);

      socket.onopen = () => {
        reconnectDelay = 1000;
      };

      socket.onmessage = (event) => {
        try {
          const parsed = JSON.parse(event.data) as RechargeOrderSocketEvent;
          if (!parsed?.order || parsed.order.id !== orderId) {
            return;
          }
          setOrder(parsed.order);
        } catch {
          // Ignore malformed websocket events.
        }
      };

      socket.onerror = () => {
        socket?.close();
      };

      socket.onclose = () => {
        if (closed) {
          return;
        }
        reconnectTimer = setTimeout(connect, reconnectDelay);
        reconnectDelay = Math.min(reconnectDelay * 2, 10000);
      };
    };

    connect();

    return () => {
      closed = true;
      setPolling(false);
      if (reconnectTimer) {
        clearTimeout(reconnectTimer);
      }
      if (socket) {
        socket.close();
      }
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, order?.id, order?.status]);

  useEffect(() => {
    if (!order) {
      return;
    }

    if (order.status === "paid") {
      setPolling(false);
      if (successHandledOrderIdRef.current !== order.id) {
        successHandledOrderIdRef.current = order.id;
        onSuccess?.();
      }
      return;
    }

    if (order.status === "expired" || order.status === "cancelled") {
      setPolling(false);
      const toastKey = `${order.id}:${order.status}`;
      if (terminalToastKeyRef.current !== toastKey) {
        terminalToastKeyRef.current = toastKey;
        toast.error("订单已取消或过期");
      }
    }
  }, [order, onSuccess]);

  function handleClose() {
    setOrder(null);
    setPolling(false);
    successHandledOrderIdRef.current = null;
    terminalToastKeyRef.current = null;
    setCustomAmount("");
    setSelectedAmount("100.00");
    onClose();
  }

  function formatAmount(amount: string): string {
    return `¥${parseFloat(amount).toFixed(2)}`;
  }

  if (!open) {
    return null;
  }

  // Show payment status if order exists
  if (order) {
    return (
      <Modal open={open} title="充值订单" onClose={handleClose}>
        <div className="space-y-4">
          <div className="border-border bg-background/50 space-y-2 rounded-lg border p-4">
            <p className="flex justify-between text-sm">
              <span className="text-muted-foreground">订单号</span>
              <span className="font-mono text-xs">{order.out_trade_no}</span>
            </p>
            <p className="flex justify-between text-sm">
              <span className="text-muted-foreground">充值金额</span>
              <span className="font-semibold">{formatAmount(order.amount)}</span>
            </p>
            <p className="flex justify-between text-sm">
              <span className="text-muted-foreground">状态</span>
              <span className={order.status === "paid" ? "text-green-500" : "text-amber-500"}>
                {order.status === "pending" && "等待支付"}
                {order.status === "paid" && "支付成功"}
                {order.status === "expired" && "已过期"}
                {order.status === "cancelled" && "已取消"}
              </span>
            </p>
          </div>

          {order.status === "pending" && (
            <div className="space-y-3">
              <p className="text-muted-foreground text-center text-sm">
                {polling ? "正在等待支付完成..." : "正在建立实时连接..."}
              </p>
            </div>
          )}

          {order.status === "paid" && (
            <div className="space-y-2 text-center">
              <p className="font-medium text-green-500">充值成功！</p>
              <Button className="w-full" onClick={handleClose}>
                完成
              </Button>
            </div>
          )}
        </div>
      </Modal>
    );
  }

  // Show amount selection
  return (
    <Modal open={open} title="充值余额" description="选择充值金额" onClose={handleClose}>
      <div className="space-y-4">
        <div className="grid grid-cols-2 gap-2">
          {PRESET_AMOUNTS.map((preset) => (
            <button
              key={preset.value}
              type="button"
              onClick={() => {
                setSelectedAmount(preset.value);
                setCustomAmount("");
              }}
              className={`rounded-md border px-4 py-3 text-sm font-medium transition-colors ${
                selectedAmount === preset.value && !customAmount
                  ? "border-rose-500 bg-rose-500/20 text-rose-100"
                  : "border-border bg-background hover:border-rose-500/50"
              }`}
            >
              {preset.label}
            </button>
          ))}
        </div>

        <div className="space-y-2">
          <label className="text-muted-foreground text-sm">自定义金额（元）</label>
          <Input
            type="number"
            placeholder="输入金额"
            value={customAmount}
            onChange={(e) => {
              setCustomAmount(e.target.value);
            }}
            min={1}
            step={0.01}
          />
        </div>

        <Card className="border-dashed">
          <CardHeader className="py-3">
            <CardTitle className="text-sm">充值金额</CardTitle>
          </CardHeader>
          <CardContent className="py-2">
            <p className="text-2xl font-semibold">{formatAmount(getAmount())}</p>
          </CardContent>
        </Card>

        <Button className="w-full" onClick={handleCreateOrder} disabled={loading}>
          {loading ? "创建订单中..." : "确认充值"}
        </Button>
      </div>
    </Modal>
  );
}
