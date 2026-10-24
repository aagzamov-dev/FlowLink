import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { Updates } from "./Updates";
import { initialUpdate, type UpdateStatus } from "../updates";

function render(status: Partial<UpdateStatus>, desktop = true) {
  return renderToStaticMarkup(
    <Updates
      update={{ ...initialUpdate, ...status }}
      desktop={desktop}
      automatic
      onAutomatic={() => {}}
      run={async () => {}}
    />,
  );
}

describe("update controls", () => {
  it("requires a verified download before offering install and restart", () => {
    expect(render({ phase: "available", version: "0.2.1" })).toContain(
      "Download update",
    );
    const downloading = render({
      phase: "downloading",
      total: null,
      downloaded: 100,
    });
    expect(downloading).toContain('disabled=""');
    expect(downloading).not.toContain("Install &amp; restart");
    expect(downloading).not.toContain('value="0"');
    expect(render({ phase: "ready" })).toContain("Install &amp; restart");
  });
  it("shows retry errors without claiming success and escapes release notes", () => {
    const html = render({
      phase: "error",
      error: "Feed unavailable",
      notes: "<script>bad()</script>",
    });
    expect(html).toContain('role="alert"');
    expect(html).toContain("Feed unavailable");
    expect(html).not.toContain("You’re up to date");
    expect(html).not.toContain("<script>");
    expect(html).toContain("&lt;script&gt;");
    expect(
      render({ phase: "available", error: "Verification failed" }),
    ).toContain("Check again");
  });
  it("disables update actions in the browser preview and during installation", () => {
    expect(render({}, false)).toContain('disabled=""');
    expect(render({ phase: "installing" })).toContain('disabled=""');
  });
});
