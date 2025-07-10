export type InspectApi = {
  queue: QueueItem[];
  player: PlayerState | null;
};

export type PlayerState = {
  state: "playing" | "paused";
  time: number;
  length: number;
  volume: number;
};

export type QueueItem = {
  job_id: string;
  current: boolean;
  track_info: TrackInfo;
};

export type TrackInfo = {
  title: string;
  channel: string;
  uploader_id: string;
  acodec: string;
  vcodec: string;
  height: number | null;
  width: number | null;
  thumbnail: string;
  track_type: "merged" | "split";
};

export type VideoType = "merged" | "split";
