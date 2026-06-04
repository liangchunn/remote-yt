import { type ImgHTMLAttributes, useState } from "react";

type SafeImageProps = Omit<ImgHTMLAttributes<HTMLImageElement>, "src"> & {
  src: string;
  fallbackSrc?: string;
};

export function SafeImage({ src, fallbackSrc, ...props }: SafeImageProps) {
  return (
    <SafeImageInner key={src} src={src} fallbackSrc={fallbackSrc} {...props} />
  );
}

function SafeImageInner({
  src,
  fallbackSrc,
  alt = "",
  onError,
  ...props
}: SafeImageProps) {
  const [currentSrc, setCurrentSrc] = useState(src);
  const [hidden, setHidden] = useState(false);

  if (hidden) return null;

  return (
    <img
      {...props}
      src={currentSrc}
      alt={alt}
      onError={(event) => {
        onError?.(event);

        if (fallbackSrc && currentSrc !== fallbackSrc) {
          setCurrentSrc(fallbackSrc);
        } else {
          setHidden(true);
        }
      }}
    />
  );
}
