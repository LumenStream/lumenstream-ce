import { Button, buttonVariants } from "@/components/ui/button";
import type { InviteSummary } from "@/lib/types/admin";
import type { Playlist } from "@/lib/types/playlist";

interface SocialSectionProps {
  inviteSummary: InviteSummary | null;
  inviteResetting: boolean;
  onResetInviteCode: () => void;
  onCopyText: (text: string, tip: string) => void;
  playlists: Playlist[];
  playlistActionId: string | null;
  onTogglePlaylistPublic: (playlist: Playlist) => void;
  onDeletePlaylist: (id: string) => void;
  onReloadPlaylists: () => void;
}

export function SocialSection({
  inviteSummary,
  inviteResetting,
  onResetInviteCode,
  onCopyText,
  playlists,
  playlistActionId,
  onTogglePlaylistPublic,
  onDeletePlaylist,
  onReloadPlaylists,
}: SocialSectionProps) {
  const inviteCode = inviteSummary?.code || "------------";

  return (
    <div className="space-y-6">
      {/* Invite code section */}
      <div className="border-border/50 border-b pb-5">
        <h3 className="text-muted-foreground mb-4 text-xs font-semibold tracking-wide uppercase">
          邀请码
        </h3>
        <p className="text-muted-foreground mb-3 text-xs">可复制分享，邀请新用户体验服务。</p>
        <div className="flex flex-wrap items-center gap-3">
          <code className="font-mono text-lg font-semibold tracking-widest">{inviteCode}</code>
          <Button variant="outline" onClick={() => onCopyText(inviteCode, "邀请码已复制。")}>
            复制
          </Button>
          <Button variant="outline" disabled={inviteResetting} onClick={onResetInviteCode}>
            {inviteResetting ? "重置中..." : "重置"}
          </Button>
        </div>
        {inviteSummary && (
          <div className="mt-4 space-y-1">
            <div className="flex justify-between text-sm">
              <span className="text-muted-foreground">已邀请</span>
              <span className="font-medium">{inviteSummary.invited_count} 人</span>
            </div>
            {typeof inviteSummary.rebate_total === "string" ? (
              <div className="flex justify-between text-sm">
                <span className="text-muted-foreground">累计返利</span>
                <span className="font-medium">
                  ¥{Number.parseFloat(inviteSummary.rebate_total).toFixed(2)}
                </span>
              </div>
            ) : null}
            {inviteSummary.invitee_bonus_enabled ? (
              <p className="text-muted-foreground pt-1 text-xs">新用户注册可获赠送余额。</p>
            ) : null}
          </div>
        )}
      </div>

      {/* Playlists section */}
      <PlaylistsSection
        playlists={playlists}
        playlistActionId={playlistActionId}
        onTogglePlaylistPublic={onTogglePlaylistPublic}
        onDeletePlaylist={onDeletePlaylist}
        onReloadPlaylists={onReloadPlaylists}
      />
    </div>
  );
}

function PlaylistsSection({
  playlists,
  playlistActionId,
  onTogglePlaylistPublic,
  onDeletePlaylist,
  onReloadPlaylists,
}: {
  playlists: Playlist[];
  playlistActionId: string | null;
  onTogglePlaylistPublic: (playlist: Playlist) => void;
  onDeletePlaylist: (id: string) => void;
  onReloadPlaylists: () => void;
}) {
  return (
    <div>
      <div className="mb-4 flex items-center justify-between">
        <h3 className="text-muted-foreground text-xs font-semibold tracking-wide uppercase">
          收藏夹
        </h3>
        <Button variant="outline" size="sm" onClick={onReloadPlaylists}>
          刷新
        </Button>
      </div>
      <p className="text-muted-foreground mb-3 text-xs">
        可切换公开/私有，并在不需要时删除收藏夹。
      </p>

      {playlists.length === 0 ? (
        <p className="text-muted-foreground text-sm">
          你还没有收藏夹。可在海报卡&ldquo;添加到列表&rdquo;中创建。
        </p>
      ) : (
        <div className="space-y-0">
          {playlists.map((playlist) => (
            <div
              key={playlist.id}
              className="border-border/30 flex items-center justify-between border-b py-2.5 last:border-0"
            >
              <div className="space-y-0.5">
                <p className="text-sm font-medium">{playlist.name}</p>
                <p className="text-muted-foreground text-xs">
                  {playlist.item_count} 项 · {playlist.is_public ? "公开" : "私有"}
                  {playlist.description ? ` · ${playlist.description}` : ""}
                </p>
              </div>
              <div className="flex gap-2">
                <Button
                  variant="outline"
                  size="sm"
                  disabled={playlistActionId === playlist.id}
                  onClick={() => onTogglePlaylistPublic(playlist)}
                >
                  {playlist.is_public ? "设为私有" : "设为公开"}
                </Button>
                <Button
                  variant="outline"
                  size="sm"
                  disabled={playlistActionId === playlist.id}
                  onClick={() => onDeletePlaylist(playlist.id)}
                >
                  删除
                </Button>
              </div>
            </div>
          ))}
        </div>
      )}

      <div className="mt-4">
        <a href="/app/playlists" className={buttonVariants({ variant: "outline" })}>
          进入收藏夹中心
        </a>
      </div>
    </div>
  );
}
