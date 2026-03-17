import { useMemo, useState } from "react";

import { Modal } from "@/components/domain/Modal";
import { Button } from "@/components/ui/button";
import { detectPlatform, getPlayersForPlatform } from "@/lib/player/deeplink";

interface PlayerPickerModalProps {
  open: boolean;
  onClose: () => void;
  streamUrl: string;
  title: string;
}

export function PlayerPickerModal({ open, onClose, streamUrl, title }: PlayerPickerModalProps) {
  const [copied, setCopied] = useState(false);

  const players = useMemo(() => {
    const ua = typeof navigator === "undefined" ? "" : navigator.userAgent;
    return getPlayersForPlatform(detectPlatform(ua));
  }, []);

  function handlePlayerClick(buildUrl: (s: string, t: string) => string) {
    window.location.assign(buildUrl(streamUrl, title));
    onClose();
  }

  async function handleCopy() {
    try {
      await navigator.clipboard.writeText(streamUrl);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // Silently fail — button text already indicates the action.
    }
  }

  return (
    <Modal
      open={open}
      onClose={onClose}
      title="选择播放器"
      description="选择播放器进行播放"
      showFooterClose={false}
      showHeaderClose
    >
      <div className="space-y-4">
        <div className="flex flex-wrap justify-center gap-4">
          {players.map((player) => (
            <button
              key={player.id}
              type="button"
              className="group flex w-[72px] flex-col items-center gap-1.5"
              onClick={() => handlePlayerClick(player.buildUrl)}
            >
              <div className="relative h-[56px] w-[56px] overflow-hidden rounded-2xl shadow-md transition-transform group-hover:scale-110">
                <img
                  src={`/logo/${player.id}.webp`}
                  alt={player.name}
                  className="h-full w-full object-cover"
                />
                {player.recommended ? (
                  <span className="absolute -top-0.5 -right-0.5 flex h-4 w-4 items-center justify-center rounded-full bg-green-500 text-[8px] font-bold text-white shadow">
                    ★
                  </span>
                ) : null}
              </div>
              <span className="line-clamp-1 text-[11px] text-neutral-300 group-hover:text-white">
                {player.name}
              </span>
            </button>
          ))}
        </div>

        <hr className="border-white/10" />

        <Button variant="secondary" className="w-full gap-2" onClick={() => void handleCopy()}>
          <svg
            xmlns="http://www.w3.org/2000/svg"
            viewBox="0 0 20 20"
            fill="currentColor"
            className="h-4 w-4"
            aria-hidden="true"
          >
            <path d="M7 3.5A1.5 1.5 0 018.5 2h3.879a1.5 1.5 0 011.06.44l3.122 3.12A1.5 1.5 0 0117 6.622V12.5a1.5 1.5 0 01-1.5 1.5h-1v-3.379a3 3 0 00-.879-2.121L10.5 5.379A3 3 0 008.379 4.5H7v-1z" />
            <path d="M4.5 6A1.5 1.5 0 003 7.5v9A1.5 1.5 0 004.5 18h7a1.5 1.5 0 001.5-1.5v-5.879a1.5 1.5 0 00-.44-1.06L9.44 6.439A1.5 1.5 0 008.378 6H4.5z" />
          </svg>
          {copied ? "已复制" : "复制播放链接"}
        </Button>
      </div>
    </Modal>
  );
}
