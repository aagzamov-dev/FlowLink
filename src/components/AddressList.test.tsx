import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { preview } from "../types";
import { AddressList } from "./AddressList";

function render(port: number, ip = "192.168.1.13") {
  return renderToStaticMarkup(
    <AddressList
      state={{
        ...preview,
        port,
        addresses: [{ ip, interface: "Ethernet", kind: "Ethernet" }],
      }}
      copy={async () => {}}
      compact
    />,
  );
}

describe("local connection addresses", () => {
  it("shows the fallback port for manual connections, including IPv6", () => {
    expect(render(54321)).toContain("192.168.1.13:54321");
    expect(render(54321)).toContain("Using TCP port 54321");
    expect(render(54321)).not.toContain('title="Copy IP address"');
    expect(render(54321, "fd00::1")).toContain("[fd00::1]:54321");
  });

  it("disables copying until a listener is available", () => {
    expect(render(0)).toContain("Waiting for a connection port");
    expect(render(0).match(/disabled=""/g)).toHaveLength(2);
    expect(render(47831)).not.toContain('disabled=""');
    expect(render(47831)).toContain('title="Copy IP address"');
  });
});
