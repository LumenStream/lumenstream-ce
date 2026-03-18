import React, { useState } from "react";
import { ArrowDown, ArrowUp, Check } from "lucide-react";

import { Modal } from "@/components/domain/Modal";
import { Button } from "@/components/ui/button";
import {
  SCRAPER_LIBRARY_ROUTE_KEYS,
  getScraperLibraryRouteLabel,
} from "@/lib/admin/scraper-policy";

interface ScraperPolicyModalProps {
  open: boolean;
  onClose: () => void;
  onSave: (policy: Record<string, string[]>) => void;
  initialPolicy: Record<string, string[]>;
  availableProviders: string[];
  title?: string;
}

export function ScraperPolicyModal({
  open,
  onClose,
  onSave,
  initialPolicy,
  availableProviders,
  title = "配置刮削链路",
}: ScraperPolicyModalProps) {
  const [policy, setPolicy] = useState<Record<string, string[]>>(initialPolicy);
  const [activeScenario, setActiveScenario] = useState<string>(SCRAPER_LIBRARY_ROUTE_KEYS[0]);

  const currentProviders = policy[activeScenario] || [];

  function toggleProvider(provider: string) {
    const next = { ...policy };
    const current = next[activeScenario] || [];
    if (current.includes(provider)) {
      next[activeScenario] = current.filter((p) => p !== provider);
    } else {
      next[activeScenario] = [...current, provider];
    }
    setPolicy(next);
  }

  function moveUp(provider: string) {
    const next = { ...policy };
    const current = [...(next[activeScenario] || [])];
    const index = current.indexOf(provider);
    if (index > 0) {
      [current[index - 1], current[index]] = [current[index], current[index - 1]];
      next[activeScenario] = current;
      setPolicy(next);
    }
  }

  function moveDown(provider: string) {
    const next = { ...policy };
    const current = [...(next[activeScenario] || [])];
    const index = current.indexOf(provider);
    if (index < current.length - 1) {
      [current[index], current[index + 1]] = [current[index + 1], current[index]];
      next[activeScenario] = current;
      setPolicy(next);
    }
  }

  function handleSave() {
    onSave(policy);
    onClose();
  }

  return (
    <Modal open={open} title={title} onClose={onClose} showHeaderClose>
      <div className="space-y-4">
        <div className="flex gap-2 overflow-x-auto pb-2">
          {SCRAPER_LIBRARY_ROUTE_KEYS.map((scenario) => (
            <button
              key={scenario}
              onClick={() => setActiveScenario(scenario)}
              className={`shrink-0 rounded px-3 py-1.5 text-sm transition ${
                activeScenario === scenario
                  ? "bg-primary text-primary-foreground"
                  : "bg-secondary text-secondary-foreground hover:bg-secondary/80"
              }`}
            >
              {getScraperLibraryRouteLabel(scenario)}
            </button>
          ))}
        </div>

        <div className="space-y-2">
          <p className="text-xs text-slate-500">选择并排序 provider，从上到下依次回退</p>
          {availableProviders.map((provider) => {
            const isSelected = currentProviders.includes(provider);
            const index = currentProviders.indexOf(provider);
            return (
              <div
                key={provider}
                className="flex items-center gap-2 rounded border border-white/10 bg-white/[0.02] p-3"
              >
                <button
                  onClick={() => toggleProvider(provider)}
                  className={`flex h-5 w-5 shrink-0 items-center justify-center rounded border transition ${
                    isSelected
                      ? "border-primary bg-primary text-primary-foreground"
                      : "border-white/20 bg-white/5"
                  }`}
                >
                  {isSelected && <Check className="h-3.5 w-3.5" />}
                </button>
                <span className="flex-1 text-sm text-white">{provider}</span>
                {isSelected && (
                  <div className="flex gap-1">
                    <span className="text-xs text-slate-500">#{index + 1}</span>
                    <button
                      onClick={() => moveUp(provider)}
                      disabled={index === 0}
                      className="rounded p-1 text-slate-400 hover:bg-white/10 hover:text-white disabled:opacity-30"
                    >
                      <ArrowUp className="h-3.5 w-3.5" />
                    </button>
                    <button
                      onClick={() => moveDown(provider)}
                      disabled={index === currentProviders.length - 1}
                      className="rounded p-1 text-slate-400 hover:bg-white/10 hover:text-white disabled:opacity-30"
                    >
                      <ArrowDown className="h-3.5 w-3.5" />
                    </button>
                  </div>
                )}
              </div>
            );
          })}
        </div>

        <div className="flex justify-end gap-2 pt-2">
          <Button variant="secondary" onClick={onClose}>
            取消
          </Button>
          <Button onClick={handleSave}>保存配置</Button>
        </div>
      </div>
    </Modal>
  );
}
