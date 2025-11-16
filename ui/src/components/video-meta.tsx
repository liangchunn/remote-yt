import type { TrackInfo } from "@/types/inspect";

export function VideoMeta({
  acodec,
  vcodec,
  track_type,
  width,
  height,
}: Pick<TrackInfo, "acodec" | "vcodec" | "track_type" | "width" | "height">) {
  const dimensions = width && height ? `${width}×${height}` : null;
  return (
    <>
      <Badge label={dimensions} />
      <Codec acodec={acodec} track_type={track_type} vcodec={vcodec} />
    </>
  );
}

function Codec({
  acodec,
  vcodec,
  track_type,
}: Pick<TrackInfo, "acodec" | "vcodec" | "track_type">) {
  const acodecTrimmed = trimFormat(acodec);
  const vcodecTrimmed = trimFormat(vcodec);
  if (track_type === "merged") {
    const combined = [acodecTrimmed, vcodecTrimmed].filter(Boolean).join("+");
    return <Badge label={combined} />;
  }
  if (track_type === "split") {
    return (
      <>
        <Badge label={vcodecTrimmed} />
        <Badge label={acodecTrimmed} />
      </>
    );
  }
}

function Badge({ label }: { label: string | null }) {
  if (!label) {
    return null;
  }
  return (
    <div className="text-xs font-mono py-0.5 px-1 border rounded-sm inline-block text-secondary-foreground bg-background/75">
      {label}
    </div>
  );
}

function trimFormat(codec: string): string | null {
  if (codec.length === 0) {
    return null;
  }
  if (codec.includes(".")) {
    return codec.split(".")[0];
  } else {
    return codec;
  }
}
