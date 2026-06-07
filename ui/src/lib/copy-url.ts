import copy from "copy-to-clipboard";
import { toast } from "sonner";

export async function copyUrlToClipboard(url: string) {
  const copied = await copy(url, { fallbackToPrompt: true }).catch(() => false);

  if (copied) {
    toast.success("URL copied");
  } else {
    toast.error("Could not copy URL");
  }
}
