import React, { useMemo, useState } from "react";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { authenticateByName } from "@/lib/api/auth";
import type { ApiError } from "@/lib/api/client";
import { setAccessToken, setCurrentUser } from "@/lib/auth/token";
import { isMockFeatureEnabled } from "@/lib/mock/mode";
import { enableMockExperience } from "@/lib/mock/session";
import { toast } from "@/lib/notifications/toast-store";

interface InputFieldProps {
  id: string;
  label: string;
  type?: string;
  autoComplete: string;
  value: string;
  onChange: (value: string) => void;
  disabled?: boolean;
  required?: boolean;
}

function AnimatedInputField({
  id,
  label,
  type = "text",
  autoComplete,
  value,
  onChange,
  disabled,
  required,
}: InputFieldProps) {
  const [isFocused, setIsFocused] = useState(false);

  return (
    <div className="space-y-2">
      <label
        htmlFor={id}
        className={`text-sm font-medium transition-colors duration-200 ${
          isFocused ? "text-primary" : "text-muted-foreground"
        }`}
      >
        {label}
      </label>
      <div className="relative">
        <Input
          id={id}
          type={type}
          autoComplete={autoComplete}
          value={value}
          onChange={(event) => onChange(event.target.value)}
          onFocus={() => setIsFocused(true)}
          onBlur={() => setIsFocused(false)}
          disabled={disabled}
          required={required}
          className={`transition-all duration-200 ${
            isFocused
              ? "border-primary/50 ring-primary/20 ring-2"
              : "border-border hover:border-white/20"
          }`}
        />
        <div
          className={`bg-primary absolute bottom-0 left-0 h-0.5 transition-all duration-300 ${
            isFocused ? "w-full" : "w-0"
          }`}
        />
      </div>
    </div>
  );
}

export function LoginForm() {
  const mockFeatureEnabled = isMockFeatureEnabled();
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [rememberMe, setRememberMe] = useState(false);
  const [submitting, setSubmitting] = useState(false);

  const canSubmit = useMemo(
    () => username.trim().length > 0 && password.length > 0,
    [password, username]
  );

  async function onSubmit(event: React.SubmitEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!canSubmit || submitting) {
      return;
    }

    setSubmitting(true);
    try {
      const result = await authenticateByName(username.trim(), password);
      const storage = { persistent: rememberMe };
      setAccessToken(result.AccessToken, storage);
      setCurrentUser(result.User, storage);
      window.location.replace("/app/home");
    } catch (cause) {
      const apiError = cause as ApiError;
      toast.error(apiError.message || "登录失败");
    } finally {
      setSubmitting(false);
    }
  }

  async function onDemoMode() {
    if (!mockFeatureEnabled) {
      toast.warning("当前部署未启用演示模式。");
      return;
    }

    setSubmitting(true);
    try {
      await enableMockExperience("demo-admin", { persistent: rememberMe });
      window.location.replace("/app/home");
    } catch {
      toast.error("演示模式启动失败，请刷新重试。");
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <div className="animate-fade-in-up light:border-black/[0.08] light:bg-white/80 light:shadow-[0_20px_60px_-15px_rgba(0,0,0,0.12)] w-full max-w-md rounded-2xl border border-white/[0.06] bg-black/40 shadow-[0_20px_60px_-15px_rgba(0,0,0,0.5)] backdrop-blur-xl">
      <div className="space-y-1 p-6">
        <h2 className="text-2xl font-bold tracking-tight">登录 LumenStream Web</h2>
        <p className="text-muted-foreground text-sm">输入账号后进入元信息浏览与管理控制台</p>
      </div>
      <div className="px-6 pb-6">
        <form className="space-y-5" onSubmit={onSubmit}>
          <AnimatedInputField
            id="username"
            label="用户名"
            autoComplete="username"
            value={username}
            onChange={setUsername}
            disabled={submitting}
            required
          />

          <AnimatedInputField
            id="password"
            label="密码"
            type="password"
            autoComplete="current-password"
            value={password}
            onChange={setPassword}
            disabled={submitting}
            required
          />

          <label className="text-muted-foreground flex items-center gap-2 text-sm">
            <input
              type="checkbox"
              checked={rememberMe}
              onChange={(event) => setRememberMe(event.target.checked)}
              disabled={submitting}
            />
            保持登录
          </label>

          <div className="flex flex-col gap-3 pt-2">
            <Button
              className="w-full transition-all duration-200 hover:scale-[1.02] active:scale-[0.98]"
              type="submit"
              variant="immersive"
              disabled={!canSubmit || submitting}
            >
              {submitting ? (
                <span className="flex items-center gap-2">
                  <svg className="h-4 w-4 animate-spin" fill="none" viewBox="0 0 24 24">
                    <circle
                      className="opacity-25"
                      cx="12"
                      cy="12"
                      r="10"
                      stroke="currentColor"
                      strokeWidth="4"
                    />
                    <path
                      className="opacity-75"
                      fill="currentColor"
                      d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
                    />
                  </svg>
                  登录中...
                </span>
              ) : (
                <span className="flex items-center gap-2">
                  <svg className="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M11 16l-4-4m0 0l4-4m-4 4h14m-5 4v1a3 3 0 01-3 3H6a3 3 0 01-3-3V7a3 3 0 013-3h7a3 3 0 013 3v1"
                    />
                  </svg>
                  登录
                </span>
              )}
            </Button>
            {mockFeatureEnabled ? (
              <Button
                className="w-full transition-all duration-200 hover:scale-[1.02] active:scale-[0.98]"
                type="button"
                variant="secondary"
                onClick={() => void onDemoMode()}
                disabled={submitting}
              >
                <span className="flex items-center gap-2">
                  <svg className="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z"
                    />
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
                    />
                  </svg>
                  演示模式（无需后端）
                </span>
              </Button>
            ) : null}
            <a
              className="text-muted-foreground text-center text-xs hover:underline"
              href="/register"
            >
              没有账号？使用邀请码注册
            </a>
          </div>
        </form>
      </div>
    </div>
  );
}
