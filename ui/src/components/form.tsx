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
import type { VideoType } from "@/types/queue";

const QUALITY_TO_MIN_HEIGHT = {
  sd: 480,
  hd: 720,
  fhd: 1080,
  sd_s: 480,
};

export function Form({
  mutation,
}: {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  mutation: UseMutationResult<any, Error, [VideoType, string, number], unknown>;
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
    mutation.mutate([quality === "sd_s" ? "split" : "merged", url, min_height]);
  };
  return (
    <form className="flex gap-2" onSubmit={handleSubmit}>
      <Input
        id="url"
        placeholder="Insert URL..."
        defaultValue={"https://www.youtube.com/watch?v=GNXNwT65ymg"}
      />
      <Select defaultValue="sd" name="quality">
        <SelectTrigger className="w-[110px]">
          <SelectValue placeholder="Theme" />
        </SelectTrigger>
        <SelectContent className="min-w-0">
          <SelectItem value="sd">SD</SelectItem>
          <SelectItem value="hd">HD</SelectItem>
          <SelectItem value="fhd">FHD</SelectItem>
          <SelectItem value="sd_s">SD (Split)</SelectItem>
        </SelectContent>
      </Select>

      <Button>Add</Button>
    </form>
  );
}
