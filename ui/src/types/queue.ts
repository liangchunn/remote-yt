export type QueueApi = QueueItem[];

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
