import { usePlayerCommandsMutation } from "@/lib/commands";
import type { PlayerState } from "@/types/inspect";
import { Volume1, Volume2, VolumeX } from "lucide-react";
import { useState, useEffect, useRef } from "react";
import { Button } from "./ui/button";
import { formatTime } from "@/lib/format-time";
import { Popover, PopoverContent, PopoverTrigger } from "./ui/popover";
import { Slider } from "./ui/slider";

export function PlayerProgress({
  playerState,
}: {
  playerState: PlayerState | null;
}) {
  const [shouldHide, setShouldHide] = useState(!playerState);

  useEffect(() => {
    if (!playerState) {
      // Small delay to allow fade-out before hiding
      const timeout = setTimeout(() => setShouldHide(true), 300);
      return () => clearTimeout(timeout);
    }
    // When playerState exists, clear the hide state immediately
    const timeout = setTimeout(() => setShouldHide(false), 0);
    return () => clearTimeout(timeout);
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
      usableWidth,
    );
    const percent = offsetX / usableWidth;
    const newTime = Math.round(percent * playerState.length);
    setDragTime(newTime);
  };

  const currentTime =
    isDragging && dragTime !== null ? dragTime : playerState?.time || 0;
  const progressPercent = (currentTime / (playerState?.length || 1)) * 100;

  const currString = formatTime(
    isDragging && dragTime !== null ? dragTime : playerState?.time || 0,
  );
  const totalString = playerState ? formatTime(playerState.length) : "";
  const volumePercent = playerState ? toVolumePercent(playerState.volume) : 0;

  return (
    <div
      className={`transition-opacity duration-300 ease-in-out ${
        playerState ? "opacity-100" : "opacity-0"
      } ${shouldHide ? "hidden" : "block"}`}
    >
      <div className="absolute bottom-0 left-0 w-full h-24 bg-linear-to-t from-black/70 to-black/0" />
      <div className="absolute bottom-6.5 left-0 pl-4">
        <p className="tracking-tight text-sm text-white/80 font-mono">
          <span>{currString}</span>
          <span className="mx-0.5">/</span>
          <span>{totalString}</span>
        </p>
      </div>
      <div className="absolute bottom-6.5 right-0 pr-4">
        <VolumeControl volumePercent={volumePercent} />
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

function toVolumePercent(volume: number) {
  const clampedVolume = Math.min(Math.max(volume, 0), 255);
  return Math.round((clampedVolume / 255) * 100);
}

function VolumeControl({ volumePercent }: { volumePercent: number }) {
  const { setVolume } = usePlayerCommandsMutation();
  const [isOpen, setIsOpen] = useState(false);
  const [localVolume, setLocalVolume] = useState<number | null>(null);
  const displayedVolume = localVolume ?? volumePercent;

  const handleOpenChange = (open: boolean) => {
    setIsOpen(open);
    if (open) {
      setLocalVolume(null);
    }
  };

  const handleVolumeChange = (value: number | readonly number[]) => {
    const nextVolume = Array.isArray(value) ? (value[0] ?? 0) : value;
    setLocalVolume(nextVolume);
    setVolume(nextVolume);
  };

  return (
    <Popover open={isOpen} onOpenChange={handleOpenChange}>
      <PopoverTrigger
        render={
          <Button
            aria-label={`Volume ${displayedVolume}%`}
            variant="ghost"
            size="icon-sm"
            className="cursor-pointer hover:bg-muted/15 aria-expanded:bg-transparent aria-expanded:hover:bg-muted/15"
          >
            {volumePercent === 0 ? (
              <VolumeX className="stroke-white/80" />
            ) : volumePercent <= 50 ? (
              <Volume1 className="stroke-white/80" />
            ) : (
              <Volume2 className="stroke-white/80" />
            )}
          </Button>
        }
      />
      <PopoverContent
        side="top"
        align="center"
        sideOffset={2}
        className="w-10 items-center px-1 py-2 bg-background/70 backdrop-blur-lg h-36 gap-1"
      >
        <span className="text-xs font-medium tabular-nums">
          {displayedVolume}%
        </span>
        <Slider
          aria-label="Volume"
          orientation="vertical"
          min={0}
          max={100}
          value={[displayedVolume]}
          onValueChange={handleVolumeChange}
          className="data-[orientation=vertical]:h-28"
        />
      </PopoverContent>
    </Popover>
  );
}
