export type InspectApi = {
  now_playing: InspectItem | null;
  queue: InspectItem[];
  player: PlayerState | null;
};

export type PlayerState = {
  state: "playing" | "paused";
  time: number;
  length: number;
  volume: number;
};

export type InspectItem = {
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
  duration: number;
  webpage_url: string;
  job_type: JobType;
};

export type HistoryEntry = TrackInfo & {
  inserted_at: number;
};

export type JobType = "QueueMerged" | "QueueSplit" | "Queue";

export type PlaylistEntry = {
  id: string;
  title: string;
  webpage_url: string;
  thumbnail: string;
  video_count: number;
  total_duration: number;
  videos: PlaylistVideo[];
};

export type PlaylistVideo = {
  url: string;
  track_info: TrackInfo;
};
