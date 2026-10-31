/**
 * Draw a self-contained SVG (no CSS variables, no external files: an SVG
 * used as an image can resolve neither) onto a canvas and return a PNG.
 */
export async function svgToPng(svg: SVGSVGElement, scale = 2): Promise<Blob> {
  const width = svg.viewBox.baseVal?.width || svg.clientWidth || 1200;
  const height = svg.viewBox.baseVal?.height || svg.clientHeight || 850;
  const markup = new XMLSerializer().serializeToString(svg);
  const url = URL.createObjectURL(
    new Blob([markup], { type: "image/svg+xml;charset=utf-8" }),
  );
  try {
    const image = await new Promise<HTMLImageElement>((resolve, reject) => {
      const img = new Image();
      img.onload = () => resolve(img);
      img.onerror = () =>
        reject(new Error("the certificate could not be drawn"));
      img.src = url;
    });
    // Safari can report load before embedded images are decoded.
    await image.decode?.().catch(() => {});
    const canvas = document.createElement("canvas");
    canvas.width = width * scale;
    canvas.height = height * scale;
    const context = canvas.getContext("2d");
    if (!context) throw new Error("canvas is not available");
    context.drawImage(image, 0, 0, canvas.width, canvas.height);
    return await new Promise<Blob>((resolve, reject) =>
      canvas.toBlob(
        (blob) =>
          blob ? resolve(blob) : reject(new Error("PNG export failed")),
        "image/png",
      ),
    );
  } finally {
    URL.revokeObjectURL(url);
  }
}

export function download(blob: Blob, filename: string) {
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = filename;
  link.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}
