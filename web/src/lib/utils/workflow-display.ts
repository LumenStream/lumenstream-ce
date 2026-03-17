import type { AgentWorkflowStepState } from "@/lib/types/requests";

export interface UserStage {
  label: string;
  progress: number;
  description: string;
}

export function mapToUserStage(workflowSteps: AgentWorkflowStepState[]): UserStage {
  const activeStep = workflowSteps.find((s) => s.status === "active");
  const completedSteps = workflowSteps.filter((s) => s.status === "completed");

  const stepMap: Record<string, UserStage> = {
    accepted: { label: "已提交", progress: 10, description: "请求已接收，正在分析" },
    normalize: { label: "已提交", progress: 15, description: "正在标准化请求信息" },
    library_check: {
      label: "搜索中",
      progress: 30,
      description: "检查媒体库是否已有资源",
    },
    provider_search: {
      label: "搜索中",
      progress: 50,
      description: "正在全网搜索资源",
    },
    filter_dispatch: {
      label: "下载中",
      progress: 70,
      description: "筛选资源并开始下载",
    },
    verify: { label: "下载中", progress: 85, description: "验证下载结果" },
    notify: { label: "已完成", progress: 100, description: "处理完成，已通知" },
  };

  if (completedSteps.length === workflowSteps.length) {
    return { label: "已完成", progress: 100, description: "所有步骤已完成" };
  }

  return (
    stepMap[activeStep?.step || "accepted"] || {
      label: "处理中",
      progress: 5,
      description: "正在处理您的请求",
    }
  );
}
