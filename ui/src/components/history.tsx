import { formatTime, getRelativeTimeString } from "@/lib/format-time";
import type { HistoryEntry, JobType } from "@/types/inspect";
import { useQuery, type UseMutationResult } from "@tanstack/react-query";
import clsx from "clsx";
import { ChevronDown, Copy, ListEnd, Trash } from "lucide-react";
import { useLocalStorage } from "@uidotdev/usehooks";
import { toast } from "sonner";
import { Button } from "./ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "./ui/dropdown-menu";
import { useRemoveHistoryEntryMutation } from "@/lib/commands";
import { SafeImage } from "./safe-image";

export function History({
  mutation,
}: {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  mutation: UseMutationResult<any, Error, [JobType, string, number], unknown>;
}) {
  const [open, setOpen] = useLocalStorage("historyOpen", false);
  return (
    <div>
      <button
        type="button"
        className="flex w-full justify-between items-center select-none cursor-pointer group text-left"
        onClick={() => setOpen((open) => !open)}
      >
        <h1 className="text-lg font-semibold mb-1 tracking-tight group-hover:underline">
          History
        </h1>
        <ChevronDown
          className={clsx(open && "rotate-180", "size-4 transition")}
        />
      </button>
      {open && (
        <div className="flex flex-col gap-2">
          <HistoryContainer mutation={mutation} />
        </div>
      )}
    </div>
  );
}

function HistoryContainer({
  mutation,
}: {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  mutation: UseMutationResult<any, Error, [JobType, string, number], unknown>;
}) {
  const { data } = useQuery({
    queryKey: ["history"],
    queryFn: () =>
      fetch("/api/history")
        .then((res) => res.json())
        .then((data) => data as HistoryEntry[]),
  });
  const removeHistoryMutation = useRemoveHistoryEntryMutation();
  if (data) {
    return data.map((entry) => (
      <div
        key={`${entry.webpage_url}-${entry.inserted_at}`}
        className="flex items-center border rounded-md overflow-hidden gap-2 bg-white select-none"
      >
        <div className="w-36 min-h-20 self-stretch relative flex bg-muted">
          <SafeImage
            src={entry.thumbnail}
            className="h-full object-cover bg-muted"
          />

          <p className="absolute right-1 bottom-1 text-xs text-white/80 border border-black/20 rounded-sm px-0.5 bg-black/50 ">
            {formatTime(entry.duration)}
          </p>
        </div>
        <div className="flex-1 py-3 pl-1">
          <p className="leading-5 mb-0.5 line-clamp-2 ">{entry.title}</p>
          <p className="text-muted-foreground text-sm mb-0.5">
            {entry.channel}
          </p>
          <p className="text-xs text-muted-foreground">
            Played {getRelativeTimeString(entry.inserted_at)}
          </p>
        </div>
        <div className="self-start">
          <DropdownMenu>
            <DropdownMenuTrigger
              render={
                <Button variant="ghost" size="icon" className="size-8">
                  <ChevronDown />
                </Button>
              }
            />
            <DropdownMenuContent align="end" className="w-max min-w-max">
              <DropdownMenuItem
                className="whitespace-nowrap"
                onClick={() =>
                  mutation.mutate([
                    entry.job_type,
                    entry.webpage_url,
                    entry.height ?? 720,
                  ])
                }
              >
                <ListEnd className="mr-1" />
                Add to queue
              </DropdownMenuItem>
              <DropdownMenuItem
                className="whitespace-nowrap"
                onClick={() =>
                  navigator.clipboard
                    .writeText(entry.webpage_url)
                    .then(() => toast.success("URL copied"))
                }
              >
                <Copy className="mr-1" />
                Copy URL
              </DropdownMenuItem>
              <DropdownMenuItem
                variant="destructive"
                className="whitespace-nowrap"
                onClick={() => removeHistoryMutation.mutate(entry.webpage_url)}
              >
                <Trash className="mr-1" />
                Remove entry
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </div>
    ));
  } else {
    return null;
  }
}
