import { Button } from "@/components/ui/button";
import type { MePlaybackDomainsResponse } from "@/lib/types/admin";

interface PlaybackSectionProps {
  playbackDomains: MePlaybackDomainsResponse | null;
  selectedDomainId: string;
  onDomainChange: (id: string) => void;
  onSaveDomain: () => void;
  domainSaving: boolean;
  activePlaybackDomain: MePlaybackDomainsResponse["available"][number] | null;
  onCopyText: (text: string, tip: string) => void;
}

export function PlaybackSection({
  playbackDomains,
  selectedDomainId,
  onDomainChange,
  onSaveDomain,
  domainSaving,
  activePlaybackDomain,
  onCopyText,
}: PlaybackSectionProps) {
  const serverUrl = typeof window !== "undefined" ? window.location.origin : "";

  return (
    <div className="space-y-4">
      {/* Stream URL */}
      <div>
        <h3 className="text-muted-foreground mb-4 text-xs font-semibold tracking-wide uppercase">
          服务器地址
        </h3>
        <p className="text-muted-foreground mb-3 text-xs">
          Emby 兼容链接，可直接复制到Senplayer Hills等客户端使用。
        </p>
        <div className="flex gap-2">
          <p
            className="bg-muted/30 flex-1 truncate rounded-md px-3 py-2 font-mono text-sm"
            title={serverUrl}
          >
            {serverUrl}
          </p>
          <Button variant="outline" onClick={() => onCopyText(serverUrl, "服务器地址已复制。")}>
            复制地址
          </Button>
        </div>
      </div>

      {/* Domain selector */}
      {playbackDomains?.available?.length ? (
        <div className="space-y-4">
          <h3 className="text-muted-foreground text-xs font-semibold tracking-wide uppercase">
            播放域名
          </h3>
          <p className="text-muted-foreground text-xs">
            选择播放线路，不同线路可能有不同的流量倍率。
          </p>
          <div className="flex flex-wrap items-end gap-2">
            <div className="flex-1">
              <label className="text-muted-foreground mb-1 block text-xs">选择域名</label>
              <select
                className="border-input bg-background h-9 w-full rounded-md border px-3 text-sm"
                value={selectedDomainId}
                onChange={(e) => onDomainChange(e.target.value)}
              >
                {playbackDomains.available.map((item) => (
                  <option key={item.id} value={item.id}>
                    {item.name} (x{item.traffic_multiplier.toFixed(2)} · {item.base_url})
                  </option>
                ))}
              </select>
            </div>
            <Button variant="outline" disabled={domainSaving} onClick={onSaveDomain}>
              {domainSaving ? "保存中..." : "保存"}
            </Button>
          </div>
          {activePlaybackDomain && (
            <p className="text-muted-foreground text-sm">
              当前线路：{activePlaybackDomain.name}，倍率 x
              {activePlaybackDomain.traffic_multiplier.toFixed(2)}
              {activePlaybackDomain.lumenbackend_node_id
                ? `，绑定节点 ${activePlaybackDomain.lumenbackend_node_id}`
                : "，未绑定节点"}
            </p>
          )}
        </div>
      ) : null}
    </div>
  );
}
