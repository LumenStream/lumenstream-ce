import { useEffect, useState } from "react";

import { ErrorState, LoadingState } from "@/components/domain/DataState";
import { AnimatedSection } from "@/components/effects/AnimatedSection";
import { RechargeFlow } from "@/islands/billing/RechargeFlow";
import { logoutSession } from "@/lib/api/auth";
import { getWallet } from "@/lib/api/billing";
import { getMyInviteSummary, resetMyInviteCode } from "@/lib/api/invite";
import { getMyTrafficUsageByMedia } from "@/lib/api/traffic";
import { getPublicSystemCapabilities } from "@/lib/api/system";
import { getMePlaybackDomains, selectMePlaybackDomain } from "@/lib/api/items";
import { deletePlaylist, listMyPlaylists, updatePlaylist } from "@/lib/api/playlists";
import type { ApiError } from "@/lib/api/client";
import { clearAuthSession, getUserRole } from "@/lib/auth/token";
import { useAuthSession } from "@/lib/auth/use-auth-session";
import {
  getDefaultProfileSection,
  getProfileSections,
  type ProfileSection,
} from "@/lib/edition/capabilities";
import { isMockMode } from "@/lib/mock/mode";
import { disableMockExperience } from "@/lib/mock/session";
import { toast } from "@/lib/notifications/toast-store";
import type { Wallet } from "@/lib/types/billing";
import type {
  AdminSystemCapabilities,
  InviteSummary,
  MePlaybackDomainsResponse,
} from "@/lib/types/admin";
import type { MyTrafficUsageMediaSummary } from "@/lib/types/edition-commercial";
import type { Playlist } from "@/lib/types/playlist";

import { ProfileHero } from "./ProfileHero";
import { ProfileNav } from "./ProfileNav";
import { BillingTrafficSection } from "./sections/BillingTrafficSection";
import { PlaybackSection } from "./sections/PlaybackSection";
import { SocialSection } from "./sections/SocialSection";
import { TrafficRecordsSection } from "./sections/TrafficRecordsSection";

export function ProfileCenter() {
  const { session, ready } = useAuthSession();
  const [capabilities, setCapabilities] = useState<AdminSystemCapabilities | null>(null);
  const [activeSection, setActiveSection] = useState<ProfileSection>("playback");
  const [wallet, setWallet] = useState<Wallet | null>(null);
  const [trafficUsage, setTrafficUsage] = useState<MyTrafficUsageMediaSummary | null>(null);
  const [trafficSearch, setTrafficSearch] = useState("");
  const [rechargeOpen, setRechargeOpen] = useState(false);
  const [playbackDomains, setPlaybackDomains] = useState<MePlaybackDomainsResponse | null>(null);
  const [selectedDomainId, setSelectedDomainId] = useState("");
  const [playlists, setPlaylists] = useState<Playlist[]>([]);
  const [inviteSummary, setInviteSummary] = useState<InviteSummary | null>(null);
  const [inviteResetting, setInviteResetting] = useState(false);
  const [playlistActionId, setPlaylistActionId] = useState<string | null>(null);
  const [domainSaving, setDomainSaving] = useState(false);
  const [showUserId, setShowUserId] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const mockMode = isMockMode();

  useEffect(() => {
    if (!ready || !session) return;

    let cancelled = false;
    setLoading(true);

    getPublicSystemCapabilities()
      .then(async (systemCapabilities) => {
        if (cancelled) return;
        setCapabilities(systemCapabilities);
        setActiveSection((current) => {
          const sections = getProfileSections(systemCapabilities);
          return sections.includes(current)
            ? current
            : getDefaultProfileSection(systemCapabilities);
        });

        const [domainPayload, playlistPayload, invitePayload, trafficPayload, walletPayload] =
          await Promise.all([
            getMePlaybackDomains(),
            listMyPlaylists(),
            getMyInviteSummary(session.user.Id),
            systemCapabilities.advanced_traffic_controls_enabled
              ? getMyTrafficUsageByMedia()
              : Promise.resolve(null),
            systemCapabilities.billing_enabled ? getWallet() : Promise.resolve(null),
          ]);

        if (cancelled) return;
        setPlaybackDomains(domainPayload);
        setSelectedDomainId(
          domainPayload.selected_domain_id || domainPayload.default_domain_id || ""
        );
        setPlaylists(playlistPayload);
        setInviteSummary(invitePayload);
        setTrafficUsage(trafficPayload);
        setWallet(walletPayload);
        setError(null);
      })
      .catch((cause) => {
        if (cancelled) return;
        const apiError = cause as ApiError;
        setError(apiError.message || "加载个人概览失败");
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });

    return () => {
      cancelled = true;
    };
  }, [ready, session]);

  // --- Handlers ---

  async function copyText(text: string, successTip: string) {
    try {
      await navigator.clipboard.writeText(text);
      toast.success(successTip);
    } catch {
      toast.error("复制失败，请手动复制。");
    }
  }

  async function onLogout() {
    if (mockMode) {
      disableMockExperience();
      window.location.replace("/");
      return;
    }
    try {
      await logoutSession();
    } catch (cause) {
      toast.warning(`后端登出返回：${(cause as ApiError).message}`);
    } finally {
      clearAuthSession();
      window.location.replace("/login");
    }
  }

  function refreshWallet() {
    if (!capabilities?.billing_enabled) {
      setWallet(null);
      return;
    }
    getWallet()
      .then((w) => setWallet(w))
      .catch((cause) => {
        toast.error((cause as ApiError).message || "刷新钱包失败");
        setWallet(null);
      });
  }

  async function onSavePlaybackDomain() {
    if (!selectedDomainId) {
      toast.warning("请先选择播放域名。");
      return;
    }
    setDomainSaving(true);
    try {
      const result = await selectMePlaybackDomain(selectedDomainId);
      toast.success(`播放域名已切换为：${result.selected_domain_name}`);
      const latest = await getMePlaybackDomains();
      setPlaybackDomains(latest);
      setSelectedDomainId(latest.selected_domain_id || latest.default_domain_id || "");
    } catch (cause) {
      toast.error(`切换播放域名失败：${(cause as ApiError).message}`);
    } finally {
      setDomainSaving(false);
    }
  }

  async function onResetInviteCode() {
    setInviteResetting(true);
    try {
      const next = await resetMyInviteCode(session!.user.Id);
      setInviteSummary(next);
      toast.success("邀请码已重置。");
    } catch (cause) {
      toast.error(`重置邀请码失败：${(cause as ApiError).message}`);
    } finally {
      setInviteResetting(false);
    }
  }

  async function reloadPlaylists() {
    try {
      const latest = await listMyPlaylists();
      setPlaylists(latest);
    } catch (cause) {
      toast.error(`加载收藏夹失败：${(cause as ApiError).message}`);
    }
  }

  async function onTogglePlaylistPublic(playlist: Playlist) {
    setPlaylistActionId(playlist.id);
    try {
      const updated = await updatePlaylist(playlist.id, { is_public: !playlist.is_public });
      setPlaylists((cur) => cur.map((e) => (e.id === updated.id ? updated : e)));
      toast.success(updated.is_public ? "收藏夹已设为公开。" : "收藏夹已设为私有。");
    } catch (cause) {
      toast.error(`更新收藏夹失败：${(cause as ApiError).message}`);
    } finally {
      setPlaylistActionId(null);
    }
  }

  async function onDeletePlaylist(playlistId: string) {
    setPlaylistActionId(playlistId);
    try {
      await deletePlaylist(playlistId);
      setPlaylists((cur) => cur.filter((e) => e.id !== playlistId));
      toast.info("收藏夹已删除。");
    } catch (cause) {
      toast.error(`删除收藏夹失败：${(cause as ApiError).message}`);
    } finally {
      setPlaylistActionId(null);
    }
  }

  // --- Early returns ---

  if (!ready || !session) {
    return <LoadingState title="加载账户信息" description="正在读取用户会话。" />;
  }

  if (loading) {
    return <LoadingState title="加载个人概览" description="读取账户、流量与偏好设置数据。" />;
  }

  if (error) {
    return <ErrorState title="个人页加载失败" description={error} />;
  }

  // --- Derived state ---

  const role = getUserRole(session.user);
  const availableSections = getProfileSections(capabilities);
  const billingEnabled = capabilities?.billing_enabled ?? false;
  const trafficEnabled = capabilities?.advanced_traffic_controls_enabled ?? false;

  const activePlaybackDomain =
    playbackDomains?.available.find((d) => d.id === selectedDomainId) ||
    playbackDomains?.available.find((d) => d.id === playbackDomains.default_domain_id) ||
    null;
  const activeTrafficMultiplier = activePlaybackDomain?.traffic_multiplier ?? 1;

  const trafficItems = trafficUsage?.items || [];
  const trafficQuery = trafficSearch.trim().toLowerCase();
  const filteredTrafficItems = trafficQuery
    ? trafficItems.filter((item) => {
        const name = item.item_name.toLowerCase();
        const mediaId = item.media_item_id.toLowerCase();
        const itemType = item.item_type.toLowerCase();
        return (
          name.includes(trafficQuery) ||
          mediaId.includes(trafficQuery) ||
          itemType.includes(trafficQuery)
        );
      })
    : trafficItems;

  // --- Render ---

  return (
    <div className="space-y-6">
      {/* Hero — always visible */}
      <ProfileHero
        userName={session.user.Name}
        userId={session.user.Id}
        role={role}
        mockMode={mockMode}
        showUserId={showUserId}
        onToggleUserId={() => setShowUserId((v) => !v)}
        onLogout={() => void onLogout()}
      />

      {/* Mobile tabs */}
      <ProfileNav
        className="flex gap-2 overflow-x-auto pb-1 lg:hidden"
        activeSection={activeSection}
        onSectionChange={setActiveSection}
        availableSections={availableSections}
      />

      {/* Sidebar + content grid */}
      <div className="grid gap-6 lg:grid-cols-[220px_minmax(0,1fr)]">
        {/* Desktop sidebar */}
        <aside className="hidden lg:block">
          <ProfileNav
            className="flex flex-col gap-1.5"
            activeSection={activeSection}
            onSectionChange={setActiveSection}
            availableSections={availableSections}
          />
        </aside>

        {/* Active section content */}
        <div className="min-w-0">
          <AnimatedSection key={activeSection}>
            {activeSection === "billing" && billingEnabled && (
              <BillingTrafficSection
                trafficUsage={trafficUsage}
                wallet={wallet}
                activeTrafficMultiplier={activeTrafficMultiplier}
                onRechargeOpen={() => setRechargeOpen(true)}
              />
            )}

            {activeSection === "playback" && (
              <PlaybackSection
                playbackDomains={playbackDomains}
                selectedDomainId={selectedDomainId}
                onDomainChange={setSelectedDomainId}
                onSaveDomain={() => void onSavePlaybackDomain()}
                domainSaving={domainSaving}
                activePlaybackDomain={activePlaybackDomain}
                onCopyText={(t, tip) => void copyText(t, tip)}
              />
            )}

            {activeSection === "social" && (
              <SocialSection
                inviteSummary={inviteSummary}
                inviteResetting={inviteResetting}
                onResetInviteCode={() => void onResetInviteCode()}
                onCopyText={(t, tip) => void copyText(t, tip)}
                playlists={playlists}
                playlistActionId={playlistActionId}
                onTogglePlaylistPublic={(p) => void onTogglePlaylistPublic(p)}
                onDeletePlaylist={(id) => void onDeletePlaylist(id)}
                onReloadPlaylists={() => void reloadPlaylists()}
              />
            )}

            {activeSection === "traffic" && trafficEnabled && (
              <TrafficRecordsSection
                trafficSearch={trafficSearch}
                onTrafficSearchChange={setTrafficSearch}
                filteredTrafficItems={filteredTrafficItems}
              />
            )}
          </AnimatedSection>
        </div>
      </div>

      {billingEnabled ? (
        <RechargeFlow
          open={rechargeOpen}
          onClose={() => setRechargeOpen(false)}
          onSuccess={refreshWallet}
        />
      ) : null}
    </div>
  );
}
