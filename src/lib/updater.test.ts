import { describe, expect, it } from "vitest";
import { formatBytes, reduceDownloadProgress, type ProgressAccumulator } from "./updater";

describe("updater progress reducer", () => {
  it("starts with the content length reported by Tauri", () => {
    const progress = reduceDownloadProgress(
      { downloadedBytes: 0 },
      { event: "Started", data: { contentLength: 200 } },
    );

    expect(progress).toEqual({
      phase: "downloading",
      percent: 0,
      downloadedBytes: 0,
      contentLength: 200,
    });
  });

  it("calculates bounded download percentage", () => {
    const accumulator: ProgressAccumulator = { downloadedBytes: 120, contentLength: 200 };
    const progress = reduceDownloadProgress(accumulator, {
      event: "Progress",
      data: { chunkLength: 100 },
    });

    expect(progress.percent).toBe(99);
    expect(progress.downloadedBytes).toBe(220);
  });

  it("uses indeterminate progress when content length is missing", () => {
    const progress = reduceDownloadProgress(
      { downloadedBytes: 20 },
      { event: "Progress", data: { chunkLength: 40 } },
    );

    expect(progress.percent).toBeNull();
    expect(progress.downloadedBytes).toBe(60);
  });

  it("switches to installing after download finishes", () => {
    const progress = reduceDownloadProgress(
      { downloadedBytes: 200, contentLength: 200 },
      { event: "Finished" },
    );

    expect(progress.phase).toBe("installing");
    expect(progress.percent).toBe(100);
  });

  it("formats update package sizes", () => {
    expect(formatBytes()).toBe("未知大小");
    expect(formatBytes(2048)).toBe("2.0 KB");
    expect(formatBytes(2 * 1024 * 1024)).toBe("2.0 MB");
  });
});
