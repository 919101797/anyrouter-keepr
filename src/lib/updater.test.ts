import { describe, expect, it } from "vitest";
import {
  formatBytes,
  formatUpdateDetails,
  normalizeUpdaterError,
  reduceDownloadProgress,
  type ProgressAccumulator,
} from "./updater";

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

  it("keeps only product-facing update details", () => {
    const details = formatUpdateDetails(`
## 更新说明
- 优化模型选择体验
- [修复更新流程](https://github.com/example/repo/pull/1)
Full Changelog: https://github.com/example/repo/compare/v1...v2
`);

    expect(details).toBe("- 优化模型选择体验\n- 修复更新流程");
    expect(details).not.toMatch(/github|https?:\/\//i);
  });

  it("shows a concise empty update detail", () => {
    expect(formatUpdateDetails()).toBe("暂无更新详情");
  });

  it("normalizes inaccessible update metadata", () => {
    const message = normalizeUpdaterError(new Error("Could not fetch a valid release JSON from the remote"));

    expect(message).toBe("暂时无法获取更新信息，请检查网络后稍后重试。");
  });

  it("keeps unrelated updater errors intact", () => {
    expect(normalizeUpdaterError(new Error("signature validation failed"))).toBe(
      "signature validation failed",
    );
  });
});
