import { type FormEvent } from "react";
import { Button } from "./ui/button";
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
  const handleSubmit = (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    const input = e.currentTarget.elements.namedItem("url") as HTMLInputElement;
    const url = input.value;
    const quality_input = e.currentTarget.elements.namedItem(
      "quality"
    ) as HTMLSelectElement;
    const quality = quality_input.value as keyof typeof QUALITY_TO_MIN_HEIGHT;
    const min_height = QUALITY_TO_MIN_HEIGHT[quality];
    input.value = "";
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
    <form className="flex gap-2" onSubmit={handleSubmit}>
      <Input id="url" placeholder="Insert URL..." />
      <Select defaultValue="config" name="quality">
        <SelectTrigger className="w-[130px]">
          <SelectValue placeholder="Theme" />
        </SelectTrigger>
        <SelectContent className="min-w-0">
          <SelectItem value="config">Use Config</SelectItem>
          <SelectItem value="sd_s">480p</SelectItem>
          <SelectItem value="hd_s">720p</SelectItem>
          <SelectItem value="fhd_s">1080p</SelectItem>
          <SelectItem value="sd">480m</SelectItem>
        </SelectContent>
      </Select>

      <Button>Queue</Button>
    </form>
  );
}
