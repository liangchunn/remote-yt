import type { TrackInfo } from "@/types/inspect";
import { memo } from "react";

export const VideoMeta = memo(VideoMetaInner);

function VideoMetaInner({
  acodec,
  vcodec,
  track_type,
  width,
  height,
}: Pick<TrackInfo, "acodec" | "vcodec" | "track_type" | "width" | "height">) {
  return (
    <>
      <Badge label={`${width}×${height}`} />
      <Codec acodec={acodec} track_type={track_type} vcodec={vcodec} />
    </>
  );
}

function Codec({
  acodec,
  vcodec,
  track_type,
}: Pick<TrackInfo, "acodec" | "vcodec" | "track_type">) {
  if (track_type === "merged") {
    return <Badge label={`${trimFormat(vcodec)}+${trimFormat(acodec)}`} />;
  }
  if (track_type === "split") {
    return (
      <>
        <Badge label={trimFormat(vcodec)} />
        <Badge label={trimFormat(acodec)} />
      </>
    );
  }
}

function Badge({ label }: { label: string }) {
  return (
    <div className="text-xs font-mono py-0.5 px-1 border rounded-sm inline-block text-secondary-foreground bg-background/75">
      {label}
    </div>
  );
}

function trimFormat(codec: string) {
  if (codec.includes(".")) {
    return codec.split(".")[0];
  } else {
    return codec;
  }
}
