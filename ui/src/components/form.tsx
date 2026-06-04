import { type FormEvent, useState } from "react";
import { Button } from "./ui/button";
import { ClipboardPasteIcon } from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "./ui/dialog";
import { Input } from "./ui/input";
import { type UseMutationResult } from "@tanstack/react-query";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "./ui/select";
import type { JobType } from "@/types/inspect";

const QUALITY_TO_MIN_HEIGHT = {
  sd: 480,
  hd: 720,
  fhd: 1080,
  sd_s: 480,
  hd_s: 720,
  fhd_s: 1080,
  config: 0,
};

export function Form({
  mutation,
}: {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  mutation: UseMutationResult<any, Error, [JobType, string, number], unknown>;
}) {
  const [open, setOpen] = useState(false);
  const [url, setUrl] = useState("");
  const [quality, setQuality] =
    useState<keyof typeof QUALITY_TO_MIN_HEIGHT>("config");

  const pasteUrl = async () => {
    const text = await navigator.clipboard.readText();
    setUrl(text);
  };

  const handleSubmit = (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    const min_height = QUALITY_TO_MIN_HEIGHT[quality];
    setUrl("");
    setOpen(false);
    mutation.mutate([
      quality === "config"
        ? "Queue"
        : quality.endsWith("_s")
          ? "QueueSplit"
          : "QueueMerged",
      url,
      min_height,
    ]);
  };
  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button>Queue...</Button>
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Queue Media</DialogTitle>
          <DialogDescription>
            Add a URL to the playback queue.
          </DialogDescription>
        </DialogHeader>

        <form className="grid gap-4" onSubmit={handleSubmit}>
          <div className="grid gap-2">
            <label htmlFor="url" className="text-sm font-medium">
              URL
            </label>
            <div className="flex gap-2">
              <Input
                id="url"
                value={url}
                onChange={(e) => setUrl(e.target.value)}
                placeholder="Insert URL..."
              />
              <Button
                type="button"
                variant="outline"
                size="icon"
                onClick={pasteUrl}
                aria-label="Paste URL from clipboard"
              >
                <ClipboardPasteIcon />
              </Button>
            </div>
          </div>

          <div className="grid gap-2">
            <label htmlFor="quality" className="text-sm font-medium">
              Media Type
            </label>
            <Select
              value={quality}
              onValueChange={(value) =>
                setQuality(value as keyof typeof QUALITY_TO_MIN_HEIGHT)
              }
            >
              <SelectTrigger id="quality" className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="config">Auto</SelectItem>
                <SelectItem value="sd_s">480p</SelectItem>
                <SelectItem value="hd_s">720p</SelectItem>
                <SelectItem value="fhd_s">1080p</SelectItem>
                <SelectItem value="sd">480m</SelectItem>
              </SelectContent>
            </Select>
          </div>

          <DialogFooter>
            <Button type="submit">Queue</Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
