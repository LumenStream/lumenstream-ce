import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import type { MyTrafficUsageMediaSummary } from "@/lib/types/edition-commercial";
import { formatBytes } from "../utils";

const PAGE_SIZE = 50;

interface TrafficRecordsSectionProps {
  trafficSearch: string;
  onTrafficSearchChange: (value: string) => void;
  filteredTrafficItems: MyTrafficUsageMediaSummary["items"];
}

export function TrafficRecordsSection({
  trafficSearch,
  onTrafficSearchChange,
  filteredTrafficItems,
}: TrafficRecordsSectionProps) {
  const [page, setPage] = useState(0);
  const totalPages = Math.max(1, Math.ceil(filteredTrafficItems.length / PAGE_SIZE));
  const pagedItems = filteredTrafficItems.slice(page * PAGE_SIZE, (page + 1) * PAGE_SIZE);

  // Reset to first page when search changes
  const handleSearchChange = (value: string) => {
    setPage(0);
    onTrafficSearchChange(value);
  };

  return (
    <div>
      <h3 className="text-muted-foreground text-xs font-semibold tracking-wide uppercase">
        近 30 天媒体流量记录
      </h3>
      <p className="text-muted-foreground mt-1 mb-4 text-xs">
        查看在哪些媒体上消耗了流量，支持按名称、类型或 ID 搜索。
      </p>

      <div className="mb-3 flex flex-wrap items-center gap-2">
        <Input
          className="max-w-sm"
          placeholder="搜索媒体名称 / 类型 / ID"
          name="traffic-search"
          aria-label="搜索流量记录"
          value={trafficSearch}
          onChange={(e) => handleSearchChange(e.target.value)}
        />
        <Button variant="outline" onClick={() => handleSearchChange("")}>
          清空
        </Button>
      </div>

      {filteredTrafficItems.length === 0 ? (
        <p className="text-muted-foreground text-sm">
          {trafficSearch.trim() ? "未匹配到流量记录。" : "最近 30 天暂无媒体流量记录。"}
        </p>
      ) : (
        <>
          <div className="overflow-x-auto pb-4">
            <table className="w-full text-sm">
              <thead className="text-muted-foreground border-border/50 border-b text-xs">
                <tr>
                  <th className="px-3 py-3 text-left font-medium">媒体</th>
                  <th className="px-3 py-3 text-left font-medium">类型</th>
                  <th className="px-3 py-3 text-right font-medium">真实流量</th>
                  <th className="px-3 py-3 text-right font-medium">计费流量</th>
                  <th className="px-3 py-3 text-right font-medium">天数</th>
                  <th className="px-3 py-3 text-right font-medium">最近使用</th>
                </tr>
              </thead>
              <tbody className="divide-border/30 divide-y">
                {pagedItems.map((item) => (
                  <tr key={item.media_item_id} className="hover:bg-muted/30 transition-colors">
                    <td className="px-3 py-3">
                      <p className="text-foreground font-medium">{item.item_name}</p>
                      <p className="text-muted-foreground font-mono text-xs">
                        {item.media_item_id}
                      </p>
                    </td>
                    <td className="text-muted-foreground px-3 py-3">{item.item_type}</td>
                    <td className="text-foreground px-3 py-3 text-right font-medium">
                      {formatBytes(item.real_bytes_served ?? item.bytes_served)}
                    </td>
                    <td className="text-muted-foreground px-3 py-3 text-right">
                      {formatBytes(item.bytes_served)}
                    </td>
                    <td className="text-muted-foreground px-3 py-3 text-right">
                      {item.usage_days}
                    </td>
                    <td className="text-muted-foreground px-3 py-3 text-right">
                      {item.last_usage_date}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>

          {/* Pagination */}
          <div className="mt-3 flex items-center justify-between text-sm">
            <p className="text-muted-foreground text-xs">
              共 {filteredTrafficItems.length} 条，第 {page + 1}/{totalPages} 页
            </p>
            <div className="flex gap-2">
              <Button
                variant="outline"
                size="sm"
                disabled={page === 0}
                onClick={() => setPage((p) => p - 1)}
              >
                上一页
              </Button>
              <Button
                variant="outline"
                size="sm"
                disabled={page >= totalPages - 1}
                onClick={() => setPage((p) => p + 1)}
              >
                下一页
              </Button>
            </div>
          </div>
        </>
      )}
    </div>
  );
}
