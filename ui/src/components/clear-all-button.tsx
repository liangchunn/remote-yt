import { useQueueMutations } from "@/lib/commands";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from "./ui/alert-dialog";
import { Button } from "./ui/button";
import { X } from "lucide-react";

export function ClearAllButton({ show }: { show: boolean }) {
  const { clear } = useQueueMutations();

  if (!show) {
    return null;
  }
  return (
    <div className="flex justify-center">
      <AlertDialog>
        <AlertDialogTrigger>
          <Button size="sm" variant="ghost" className="text-muted-foreground">
            <X /> Clear all
          </Button>
        </AlertDialogTrigger>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>
              Clear everything and stop player?
            </AlertDialogTitle>
            <AlertDialogDescription>
              This will clear everything in the queue and stop the player. This
              action cannot be undone.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction onClick={() => clear()}>
              Clear and stop
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
