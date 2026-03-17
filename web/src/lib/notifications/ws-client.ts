import { getNotificationsWebSocketUrl, type ApiNotification } from "@/lib/api/notifications";
import { getAccessToken } from "@/lib/auth/token";

/**
 * Connection status for the WebSocket client.
 */
export type ConnectionStatus = "disconnected" | "connecting" | "connected" | "error";

/**
 * Callback for when a notification is received.
 */
export type NotificationCallback = (notification: ApiNotification) => void;

/**
 * Callback for when connection status changes.
 */
export type StatusCallback = (status: ConnectionStatus) => void;

/**
 * Options for the WebSocket client.
 */
export interface WebSocketClientOptions {
  /** Initial reconnect delay in ms (default: 1000) */
  initialReconnectDelay?: number;
  /** Maximum reconnect delay in ms (default: 30000) */
  maxReconnectDelay?: number;
  /** Reconnect delay multiplier (default: 2) */
  reconnectMultiplier?: number;
}

const DEFAULT_OPTIONS: Required<WebSocketClientOptions> = {
  initialReconnectDelay: 1000,
  maxReconnectDelay: 30000,
  reconnectMultiplier: 2,
};

/**
 * WebSocket client for real-time notification updates.
 * Supports automatic reconnection with exponential backoff.
 */
export class NotificationWebSocketClient {
  private socket: WebSocket | null = null;
  private status: ConnectionStatus = "disconnected";
  private reconnectDelay: number;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private notificationCallbacks = new Set<NotificationCallback>();
  private statusCallbacks = new Set<StatusCallback>();
  private options: Required<WebSocketClientOptions>;
  private manualDisconnect = false;

  constructor(options: WebSocketClientOptions = {}) {
    this.options = { ...DEFAULT_OPTIONS, ...options };
    this.reconnectDelay = this.options.initialReconnectDelay;
  }

  getStatus(): ConnectionStatus {
    return this.status;
  }

  onNotification(callback: NotificationCallback): () => void {
    this.notificationCallbacks.add(callback);
    return () => this.notificationCallbacks.delete(callback);
  }

  onStatusChange(callback: StatusCallback): () => void {
    this.statusCallbacks.add(callback);
    callback(this.status);
    return () => this.statusCallbacks.delete(callback);
  }

  connect(): void {
    if (this.socket || typeof WebSocket === "undefined") {
      return;
    }

    const token = getAccessToken();
    if (!token) {
      this.setStatus("error");
      return;
    }

    this.manualDisconnect = false;
    this.setStatus("connecting");

    try {
      this.socket = new WebSocket(getNotificationsWebSocketUrl(token));

      this.socket.onopen = () => {
        this.setStatus("connected");
        this.reconnectDelay = this.options.initialReconnectDelay;
      };

      this.socket.onmessage = (event) => {
        try {
          const notification = JSON.parse(event.data) as ApiNotification;
          this.notificationCallbacks.forEach((cb) => cb(notification));
        } catch {
          // Ignore malformed messages.
        }
      };

      this.socket.onerror = () => {
        this.cleanup();
        this.setStatus("error");
        this.scheduleReconnect();
      };

      this.socket.onclose = () => {
        this.cleanup();
        if (this.manualDisconnect) {
          this.setStatus("disconnected");
          return;
        }
        this.setStatus("error");
        this.scheduleReconnect();
      };
    } catch {
      this.setStatus("error");
      this.scheduleReconnect();
    }
  }

  disconnect(): void {
    this.manualDisconnect = true;
    this.cancelReconnect();
    this.cleanup();
    this.setStatus("disconnected");
  }

  reconnect(): void {
    this.disconnect();
    this.manualDisconnect = false;
    this.reconnectDelay = this.options.initialReconnectDelay;
    this.connect();
  }

  private setStatus(status: ConnectionStatus): void {
    if (this.status !== status) {
      this.status = status;
      this.statusCallbacks.forEach((cb) => cb(status));
    }
  }

  private cleanup(): void {
    if (this.socket) {
      this.socket.onopen = null;
      this.socket.onmessage = null;
      this.socket.onerror = null;
      this.socket.onclose = null;
      this.socket.close();
      this.socket = null;
    }
  }

  private cancelReconnect(): void {
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
  }

  private scheduleReconnect(): void {
    if (this.manualDisconnect) {
      return;
    }
    this.cancelReconnect();
    this.reconnectTimer = setTimeout(() => {
      this.reconnectTimer = null;
      this.connect();
    }, this.reconnectDelay);
    this.reconnectDelay = Math.min(
      this.reconnectDelay * this.options.reconnectMultiplier,
      this.options.maxReconnectDelay
    );
  }
}

let wsClient: NotificationWebSocketClient | null = null;

export function getWebSocketClient(): NotificationWebSocketClient {
  if (!wsClient) {
    wsClient = new NotificationWebSocketClient();
  }
  return wsClient;
}

export function resetWebSocketClient(): void {
  if (wsClient) {
    wsClient.disconnect();
    wsClient = null;
  }
}
