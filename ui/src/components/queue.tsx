import type { InspectApi, InspectItem } from "@/types/inspect";
import { useQuery } from "@tanstack/react-query";
import { Button } from "./ui/button";
import { LoaderCircle, Play, ChevronDown, Trash } from "lucide-react";
import {
  DndContext,
  TouchSensor,
  useSensor,
  useSensors,
  closestCenter,
  type DragEndEvent,
  MouseSensor,
  type UniqueIdentifier,
} from "@dnd-kit/core";
import {
  memo,
  useEffect,
  useMemo,
  useRef,
  useState,
  type PropsWithChildren,
} from "react";
import { useQueueMutations } from "@/lib/commands";
import {
  arrayMove,
  SortableContext,
  useSortable,
  verticalListSortingStrategy,
} from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import {
  restrictToParentElement,
  restrictToVerticalAxis,
} from "@dnd-kit/modifiers";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "./ui/dropdown-menu";
import { VideoMeta } from "./video-meta";
import { NowPlaying } from "./now-playing";
import { ClearAllButton } from "./clear-all-button";
import { formatTime } from "@/lib/format-time";

export function Queue({ isMutationPending }: { isMutationPending: boolean }) {
  const { data, error, isSuccess } = useQuery({
    queryKey: ["queue"],
    queryFn: async () => {
      const response = await fetch("/api/inspect");
      if (!response.ok) {
        throw new Error("failed to fetch");
      }
      return (await response.json()) as InspectApi;
    },
    refetchInterval: 1000,
  });

  const items = useMemo(() => data?.queue ?? [], [data]);

  const [queue, setQueue] = useState(items);

  useEffect(() => {
    if (items.length > 0) {
      setQueue(items);
    } else {
      setQueue([]);
    }
  }, [items]);

  const sensors = useSensors(
    useSensor(MouseSensor),
    useSensor(TouchSensor, {
      activationConstraint: {
        delay: 250,
        tolerance: 5,
      },
    })
  );
  const { reorder } = useQueueMutations();
  function handleDragEnd(event: DragEndEvent) {
    const { active, over } = event;

    if (over && active.id !== over.id) {
      setQueue((items) => {
        const oldIndex = items.findIndex((id) => active.id === id.job_id);
        const newIndex = items.findIndex((id) => over.id === id.job_id);

        reorder(active.id as string, newIndex);

        return arrayMove(items, oldIndex, newIndex);
      });
    }
  }

  // https://github.com/TanStack/query/discussions/6910
  const errorRef = useRef(error);
  if (error || isSuccess) errorRef.current = error;

  const nowPlaying = data?.now_playing ?? null;
  const playerState = data?.player ?? null;

  if (errorRef.current) {
    return (
      <p className="text-destructive text-center text-lg mt-4">
        Server offline
      </p>
    );
  }

  return (
    <div className="flex flex-col gap-4">
      <NowPlaying
        item={nowPlaying}
        isMutationPending={isMutationPending}
        playerState={playerState}
      />
      {(queue.length !== 0 ||
        (queue.length === 0 && !!nowPlaying && isMutationPending)) && (
        <div>
          <h1 className="text-lg font-semibold mb-1 tracking-tight">Up Next</h1>
          <div className="flex flex-col gap-2">
            <DndContext
              sensors={sensors}
              collisionDetection={closestCenter}
              onDragEnd={handleDragEnd}
              modifiers={[restrictToVerticalAxis, restrictToParentElement]}
            >
              <SortableContext
                items={queue.map((q) => q.job_id)}
                strategy={verticalListSortingStrategy}
              >
                {queue.map((item) => (
                  <DraggableItem id={item.job_id} key={item.job_id}>
                    <QueueItem item={item} />
                  </DraggableItem>
                ))}
              </SortableContext>
            </DndContext>
            {isMutationPending && <QueueItem item={null} />}
          </div>
        </div>
      )}
      {queue.length > 0 && (
        <p className="text-sm text-primary/33 text-center">
          Hint: Drag thumbnail to reorder item
        </p>
      )}
      <ClearAllButton show={!!nowPlaying || queue.length > 0} />
    </div>
  );
}

function DraggableItem({
  id,
  children,
}: PropsWithChildren<{ id: UniqueIdentifier }>) {
  const { attributes, listeners, setNodeRef, transform, transition } =
    useSortable({ id });
  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
  };

  return (
    <div ref={setNodeRef} style={style} className="relative">
      {children}
      <div
        {...attributes}
        {...listeners}
        className="absolute top-0 left-0 w-36 h-full select-none touch-none cursor-grab"
      >
        <div className="flex items-center justify-center h-full w-[calc(100%+1px)] rounded-tl-md rounded-bl-md hover:bg-white/50 active:bg-white/50 transition"></div>
      </div>
    </div>
  );
}

const QueueItem = memo(QueueItemInner);

function QueueItemInner({ item }: { item: InspectItem | null }) {
  const { cancel, swap } = useQueueMutations();
  const info = item?.track_info;
  return (
    <div className="flex items-center border rounded-md overflow-hidden gap-2 bg-white select-none">
      <div className="w-36 self-stretch relative">
        {info ? (
          <img src={info.thumbnail} className="h-full object-cover bg-muted" />
        ) : (
          <div className="w-36 object-cover bg-muted/95 ">
            <div className="aspect-video flex items-center justify-center">
              <LoaderCircle className="h-6 w-6 animate-spin text-muted-foreground" />
            </div>
          </div>
        )}
        {info && (
          <p className="absolute right-1 bottom-1 text-xs text-white/80 border border-black/20 rounded-sm px-0.5 bg-black/50 ">
            {formatTime(info.duration)}
          </p>
        )}
      </div>
      <div className="flex-1 py-3 pl-1">
        {info ? (
          <>
            <p className="leading-4 mb-0.5">{info.title}</p>
            <p className="text-muted-foreground text-sm mb-1">{info.channel}</p>
            <div className="flex items-center gap-1 top-1 right-1">
              <VideoMeta
                acodec={info.acodec}
                vcodec={info.vcodec}
                track_type={info.track_type}
                width={info.width}
                height={info.height}
              />
            </div>
          </>
        ) : (
          <p className="text-muted-foreground">Adding to queue...</p>
        )}
      </div>
      <div className="self-start">
        <DropdownMenu>
          <DropdownMenuTrigger>
            <Button
              variant="ghost"
              size="icon"
              className="size-8"
              disabled={!item}
            >
              <ChevronDown />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuItem onClick={() => item && swap(item.job_id)}>
              <Play className="mr-1" />
              Play now
            </DropdownMenuItem>
            <DropdownMenuItem
              variant="destructive"
              onClick={() => item && cancel(item.job_id)}
            >
              <Trash className="mr-1" />
              Remove item
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
    </div>
  );
}
