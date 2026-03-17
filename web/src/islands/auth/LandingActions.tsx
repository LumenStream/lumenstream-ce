import { useEffect, useState } from "react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { getAuthSession } from "@/lib/auth/token";
import { isMockFeatureEnabled, isMockMode } from "@/lib/mock/mode";
import { enableMockExperience } from "@/lib/mock/session";

export function LandingActions() {
  const mockFeatureEnabled = isMockFeatureEnabled();
  const [hasSession, setHasSession] = useState(false);
  const [mockEnabled, setMockEnabled] = useState(false);

  useEffect(() => {
    setHasSession(Boolean(getAuthSession()));
    setMockEnabled(mockFeatureEnabled && isMockMode());
  }, [mockFeatureEnabled]);

  async function startDemo() {
    await enableMockExperience("demo-admin");
    window.location.href = "/app/home";
  }

  return (
    <div className="space-y-3">
      <div className="flex flex-wrap gap-2">
        <Button onClick={() => (window.location.href = hasSession ? "/app/home" : "/login")}>
          {" "}
          {hasSession ? "继续使用" : "前往登录"}{" "}
        </Button>
        {mockFeatureEnabled ? (
          <Button variant="secondary" onClick={() => void startDemo()}>
            一键体验（Mock）
          </Button>
        ) : null}
      </div>
      <div className="text-muted-foreground flex flex-wrap gap-2 text-xs">
        {hasSession ? (
          <Badge variant="success">检测到本地登录态</Badge>
        ) : (
          <Badge variant="outline">尚未登录</Badge>
        )}
        {mockFeatureEnabled ? (
          mockEnabled ? (
            <Badge variant="secondary">Mock 模式已启用</Badge>
          ) : (
            <Badge variant="outline">Mock 模式未启用</Badge>
          )
        ) : null}
      </div>
    </div>
  );
}
