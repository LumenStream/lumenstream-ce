import React, { useMemo, useState } from "react";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { registerWithInvite } from "@/lib/api/auth";
import type { ApiError } from "@/lib/api/client";
import { setAccessToken, setCurrentUser } from "@/lib/auth/token";
import { toast } from "@/lib/notifications/toast-store";

export function RegisterForm() {
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [inviteCode, setInviteCode] = useState("");
  const [submitting, setSubmitting] = useState(false);

  const canSubmit = useMemo(
    () => username.trim().length > 0 && password.length >= 6,
    [password, username]
  );

  async function onSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!canSubmit || submitting) {
      return;
    }

    setSubmitting(true);
    try {
      const result = await registerWithInvite({
        username: username.trim(),
        password,
        invite_code: inviteCode.trim() || undefined,
      });
      setAccessToken(result.AccessToken);
      setCurrentUser(result.User);
      toast.success("注册成功，已自动登录。");
      window.location.replace("/app/home");
    } catch (cause) {
      const apiError = cause as ApiError;
      toast.error(apiError.message || "注册失败");
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <div className="animate-fade-in-up light:border-black/[0.08] light:bg-white/80 light:shadow-[0_20px_60px_-15px_rgba(0,0,0,0.12)] w-full max-w-md rounded-2xl border border-white/[0.06] bg-black/40 shadow-[0_20px_60px_-15px_rgba(0,0,0,0.5)] backdrop-blur-xl">
      <div className="space-y-1 p-6">
        <h2 className="text-2xl font-bold tracking-tight">注册 LumenStream 账户</h2>
        <p className="text-muted-foreground text-sm">使用邀请码创建新账号，注册后自动登录。</p>
      </div>
      <div className="px-6 pb-6">
        <form className="space-y-4" onSubmit={onSubmit}>
          <div className="space-y-2">
            <label className="text-muted-foreground text-sm" htmlFor="register-username">
              用户名
            </label>
            <Input
              id="register-username"
              autoComplete="username"
              value={username}
              onChange={(event) => setUsername(event.target.value)}
              disabled={submitting}
              required
            />
          </div>

          <div className="space-y-2">
            <label className="text-muted-foreground text-sm" htmlFor="register-password">
              密码（至少 6 位）
            </label>
            <Input
              id="register-password"
              type="password"
              autoComplete="new-password"
              value={password}
              onChange={(event) => setPassword(event.target.value)}
              disabled={submitting}
              required
              minLength={6}
            />
          </div>

          <div className="space-y-2">
            <label className="text-muted-foreground text-sm" htmlFor="register-invite-code">
              邀请码
            </label>
            <Input
              id="register-invite-code"
              autoComplete="off"
              value={inviteCode}
              onChange={(event) => setInviteCode(event.target.value.toUpperCase())}
              disabled={submitting}
              placeholder="若平台开启强制邀请，请填写"
            />
          </div>

          <div className="flex flex-col gap-2 pt-2">
            <Button type="submit" variant="immersive" disabled={!canSubmit || submitting}>
              {submitting ? "注册中..." : "注册并登录"}
            </Button>
            <a className="text-muted-foreground text-center text-xs hover:underline" href="/login">
              已有账号，返回登录
            </a>
          </div>
        </form>
      </div>
    </div>
  );
}
