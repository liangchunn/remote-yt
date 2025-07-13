import { usePlayerCommandsMutation } from "@/lib/commands";
import type { PlayerState } from "@/types/inspect";
import { Volume2, VolumeX } from "lucide-react";
import { useState, useEffect, useRef, memo } from "react";
import { Button } from "./ui/button";
import { formatTime } from "@/lib/format-time";

export function PlayerProgress({
  playerState,
}: {
  playerState: PlayerState | null;
}) {
  const [visible, setVisible] = useState(false);

  useEffect(() => {
    if (playerState) {
      setVisible(true);
    } else {
      // Small delay to allow fade-out before hiding
      const timeout = setTimeout(() => setVisible(false), 300);
      return () => clearTimeout(timeout);
    }
  }, [playerState]);

  const [isDragging, setIsDragging] = useState(false);
  const [dragTime, setDragTime] = useState<number | null>(null);
  const barRef = useRef<HTMLDivElement>(null);

  const { seekTo } = usePlayerCommandsMutation();

  const handlePointerDown = (e: React.PointerEvent) => {
    if (!playerState || !barRef.current) return;
    setIsDragging(true);
    barRef.current.setPointerCapture(e.pointerId);
    updateTimeFromPointer(e);
  };

  const handlePointerMove = (e: React.PointerEvent) => {
    if (!isDragging || !playerState || !barRef.current) return;
    updateTimeFromPointer(e);
  };

  const handlePointerUp = (e: React.PointerEvent) => {
    if (!playerState || !barRef.current) return;
    if (isDragging && dragTime !== null) {
      seekTo(dragTime);
    }
    setIsDragging(false);
    setDragTime(null);
    barRef.current.releasePointerCapture(e.pointerId);
  };

  const updateTimeFromPointer = (e: React.PointerEvent) => {
    if (!barRef.current || !playerState) return;
    const rect = barRef.current.getBoundingClientRect();
    const paddingX = 16; // px-4 = 1rem = 16px
    const usableWidth = rect.width - paddingX * 2;
    const offsetX = Math.min(
      Math.max(e.clientX - rect.left - paddingX, 0),
      usableWidth
    );
    const percent = offsetX / usableWidth;
    const newTime = Math.round(percent * playerState.length);
    setDragTime(newTime);
  };

  const currentTime =
    isDragging && dragTime !== null ? dragTime : playerState?.time || 0;
  const progressPercent = (currentTime / (playerState?.length || 1)) * 100;

  const currString = formatTime(
    isDragging && dragTime !== null ? dragTime : playerState?.time || 0
  );
  const totalString = playerState ? formatTime(playerState.length) : "";
  const isMuted = playerState ? playerState.volume === 0 : false;

  return (
    <div
      className={`transition-opacity duration-300 ease-in-out ${
        playerState ? "opacity-100" : "opacity-0"
      } ${visible ? "block" : "hidden"}`}
    >
      <div className="absolute bottom-0 left-0 w-full h-24 bg-gradient-to-t from-black/70 to-black/0" />
      <div className="absolute bottom-6.5 left-0 pl-4">
        <p className="tracking-tight text-sm text-white/80 font-mono">
          <span>{currString}</span>
          <span className="mx-0.5">/</span>
          <span>{totalString}</span>
        </p>
      </div>
      <div className="absolute bottom-6.5 right-0 pr-4">
        <MuteButton isMuted={isMuted} />
      </div>
      <div
        className="absolute bottom-1.5 left-0 w-full h-6 px-4 cursor-pointer touch-none"
        ref={barRef}
        onPointerDown={handlePointerDown}
        onPointerMove={handlePointerMove}
        onPointerUp={handlePointerUp}
      >
        <div className="relative h-full">
          <div className="bg-white/50 h-1 absolute left-0 top-1/2 -translate-y-1/2 w-full rounded-full"></div>
          <div
            className="bg-red-500 h-1 absolute left-0 top-1/2 -translate-y-1/2 rounded-full"
            style={{ width: `${progressPercent}%` }}
          ></div>
          <div
            className="absolute top-1/2 -translate-y-1/2 w-3 h-3 bg-red-500 rounded-full shadow"
            style={{
              left: `calc(${progressPercent}% - 6px)`, // 6px = half of 12px width
              transition: isDragging ? "none" : "left 0.1s linear",
              zIndex: 10,
            }}
          />
        </div>
      </div>
    </div>
  );
}

function MuteButtonInner({ isMuted }: { isMuted: boolean }) {
  const { fullVolume, mute } = usePlayerCommandsMutation();
  const handleVolume = () => {
    if (isMuted) {
      fullVolume();
    } else {
      mute();
    }
  };
  return (
    <Button
      variant="ghost"
      className="cursor-pointer hover:bg-muted/15 size-6"
      onClick={handleVolume}
    >
      {!isMuted && <Volume2 className="stroke-white/80" />}
      {isMuted && <VolumeX className="stroke-white/80" />}
    </Button>
  );
}

const MuteButton = memo(MuteButtonInner);
