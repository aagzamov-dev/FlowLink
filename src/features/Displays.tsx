import { useEffect, useRef, useState } from "react";
import type { PointerEvent as ReactPointerEvent } from "react";
import {
  ArrowDown,
  ArrowLeft,
  ArrowRight,
  ArrowUp,
  Monitor,
  MousePointer2,
  ShieldCheck,
} from "lucide-react";
import { isTauri } from "@tauri-apps/api/core";
import type { Placement, Rect, Snapshot, Run } from "../types";
import { layoutError, placeBeside, snap, sharedEdges } from "../layout";
import type { Side } from "../layout";
import { Empty } from "../components/common";

export function Displays({ state, run }: { state: Snapshot; run: Run }) {
  const mainId = state.controllerId ?? state.deviceId;
  const mainName =
    mainId === state.deviceId
      ? state.settings.deviceName
      : (state.devices.find((d) => d.id === mainId)?.name ?? "Main device");
  const host = state.displays.filter((d) => d.deviceId === mainId);
  const targets = state.displays.filter((d) => d.deviceId !== mainId);
  const [targetId, setTargetId] = useState("");
  const target =
    targets.find((d) => d.id === targetId) ??
    targets.find((d) => d.id === state.placements[0]?.displayId) ??
    targets[0];
  const [draft, setDraft] = useState<Placement[]>(state.placements);
  const [dragging, setDragging] = useState<{ dx: number; dy: number } | null>(
    null,
  );
  const [screenId, setScreenId] = useState("");
  const svg = useRef<SVGSVGElement>(null);
  const signature = JSON.stringify(state.placements);
  useEffect(() => {
    setDraft(JSON.parse(signature) as Placement[]);
  }, [signature, state.connectedId]);
  const selected = host.find((d) => d.id === screenId) ?? host.at(-1);
  const virtualTarget = target
    ? (draft.find((p) => p.displayId === target.id)?.rect ??
      (selected
        ? placeBeside(selected.rect, target.rect, "right")
        : target.rect))
    : null;
  const bounds = [
    ...host.map((d) => d.rect),
    ...(virtualTarget ? [virtualTarget] : []),
  ];
  const x = Math.min(...bounds.map((r) => r.x), 0) - 250;
  const y = Math.min(...bounds.map((r) => r.y), 0) - 250;
  const width = Math.max(...bounds.map((r) => r.x + r.width), 1800) - x + 250;
  const height = Math.max(...bounds.map((r) => r.y + r.height), 1100) - y + 250;
  const [view, setView] = useState<Rect | null>(null);
  const viewBox = view ?? { x, y, width, height };
  const canEdit = mainId === state.deviceId && !!state.connectedId && !!target;
  const placements =
    target && virtualTarget
      ? [{ displayId: target.id, rect: virtualTarget }]
      : [];
  const invalid = layoutError(host, placements);
  const setPosition = (rect: Rect) => {
    if (target) setDraft([{ displayId: target.id, rect }]);
  };
  const beside = (side: Side) => {
    if (selected && target) {
      setPosition(placeBeside(selected.rect, target.rect, side));
      setView(null);
    }
  };
  const point = (e: ReactPointerEvent<SVGElement>) => {
    const matrix = svg.current?.getScreenCTM();
    return matrix
      ? new DOMPoint(e.clientX, e.clientY).matrixTransform(matrix.inverse())
      : new DOMPoint();
  };
  const start = (e: ReactPointerEvent<SVGGElement>) => {
    if (!canEdit || !virtualTarget) return;
    const p = point(e);
    setView(viewBox);
    setDragging({ dx: p.x - virtualTarget.x, dy: p.y - virtualTarget.y });
    e.currentTarget.setPointerCapture(e.pointerId);
  };
  const move = (e: ReactPointerEvent<SVGGElement>) => {
    if (!dragging || !virtualTarget) return;
    const p = point(e);
    setPosition(
      snap(
        { ...virtualTarget, x: p.x - dragging.dx, y: p.y - dragging.dy },
        host,
      ),
    );
  };
  return (
    <>
      <div className="page-heading">
        <div>
          <span className="eyebrow">A NATURAL EXTENSION OF YOUR DESK</span>
          <h1>Arrange your displays</h1>
          <p>
            Place the controlled screen where it sits. Cross a shared edge to
            switch.
          </p>
        </div>
        <button onClick={() => void run("identify")} disabled={!isTauri()}>
          <Monitor size={16} />
          Identify screens
        </button>
      </div>
      <section className="display-card">
        <div className="canvas-header">
          <span>
            <i className="legend-dot windows" />
            Main device <i className="legend-dot mac" />
            Controlled screen
          </span>
          <button className="text-button" onClick={() => setView(null)}>
            Fit to view
          </button>
        </div>
        {host.length === 0 ? (
          <Empty title="Your screens will appear here">
            Open the desktop app to detect your real displays.
          </Empty>
        ) : (
          <svg
            ref={svg}
            viewBox={`${viewBox.x} ${viewBox.y} ${viewBox.width} ${viewBox.height}`}
            className="display-canvas"
            aria-label="Virtual display arrangement"
          >
            <defs>
              <pattern
                id="dots"
                width="60"
                height="60"
                patternUnits="userSpaceOnUse"
              >
                <circle cx="2" cy="2" r="2" fill="var(--canvas-dot)" />
              </pattern>
            </defs>
            <rect
              x={viewBox.x}
              y={viewBox.y}
              width={viewBox.width}
              height={viewBox.height}
              fill="url(#dots)"
            />
            {host.map((d) => (
              <g key={d.id}>
                <rect
                  className="screen-tile host"
                  x={d.rect.x}
                  y={d.rect.y}
                  width={d.rect.width}
                  height={d.rect.height}
                  rx={36}
                />
                <text
                  className="screen-number"
                  x={d.rect.x + d.rect.width / 2}
                  y={d.rect.y + d.rect.height / 2 - 20}
                  textAnchor="middle"
                >
                  {d.number}
                </text>
                <text
                  className="screen-name"
                  x={d.rect.x + d.rect.width / 2}
                  y={d.rect.y + d.rect.height / 2 + 110}
                  textAnchor="middle"
                >
                  {mainName} · {d.pixelWidth} × {d.pixelHeight}
                </text>
              </g>
            ))}
            {target && virtualTarget && (
              <g
                className={`draggable ${dragging ? "dragging" : ""}`}
                tabIndex={canEdit ? 0 : -1}
                role="slider"
                aria-label="Controlled display position. Drag or use arrow keys."
                aria-valuetext={`${Math.round(virtualTarget.x)}, ${Math.round(virtualTarget.y)}`}
                onPointerDown={start}
                onPointerMove={move}
                onPointerUp={() => {
                  setDragging(null);
                  setView(null);
                }}
                onPointerCancel={() => {
                  setDragging(null);
                  setView(null);
                }}
                onKeyDown={(e) => {
                  if (
                    !canEdit ||
                    ![
                      "ArrowLeft",
                      "ArrowRight",
                      "ArrowUp",
                      "ArrowDown",
                    ].includes(e.key)
                  )
                    return;
                  e.preventDefault();
                  const step = e.shiftKey ? 100 : 20;
                  setPosition(
                    snap(
                      {
                        ...virtualTarget,
                        x:
                          virtualTarget.x +
                          (e.key === "ArrowLeft"
                            ? -step
                            : e.key === "ArrowRight"
                              ? step
                              : 0),
                        y:
                          virtualTarget.y +
                          (e.key === "ArrowUp"
                            ? -step
                            : e.key === "ArrowDown"
                              ? step
                              : 0),
                      },
                      host,
                      12,
                    ),
                  );
                }}
              >
                <rect
                  className="screen-tile mac"
                  x={virtualTarget.x}
                  y={virtualTarget.y}
                  width={virtualTarget.width}
                  height={virtualTarget.height}
                  rx={36}
                />
                <text
                  className="screen-number"
                  x={virtualTarget.x + virtualTarget.width / 2}
                  y={virtualTarget.y + virtualTarget.height / 2 - 20}
                  textAnchor="middle"
                >
                  {target.number}
                </text>
                <text
                  className="screen-name"
                  x={virtualTarget.x + virtualTarget.width / 2}
                  y={virtualTarget.y + virtualTarget.height / 2 + 110}
                  textAnchor="middle"
                >
                  Controlled · {target.pixelWidth} × {target.pixelHeight}
                </text>
              </g>
            )}
            {virtualTarget &&
              !invalid &&
              sharedEdges(host, virtualTarget).map((edge, index) => (
                <line
                  key={index}
                  {...edge}
                  stroke="var(--green)"
                  strokeWidth={3}
                  strokeLinecap="round"
                  vectorEffect="non-scaling-stroke"
                  pointerEvents="none"
                />
              ))}
          </svg>
        )}
        <div className="canvas-footer">
          <MousePointer2 size={15} />
          <span>
            {canEdit
              ? "Drag the controlled screen. The main device’s physical arrangement stays unchanged."
              : state.connectedId
                ? `Arrange screens on ${mainName}.`
                : "Connect a device to arrange its display."}
          </span>
        </div>
      </section>
      {canEdit && (
        <section className="placement-controls">
          <div>
            <strong>Quick placement</strong>
            <small>Place controlled screen next to</small>
          </div>
          <select
            aria-label="Main screen for placement"
            value={selected?.id ?? ""}
            onChange={(e) => setScreenId(e.target.value)}
          >
            {host.map((d) => (
              <option value={d.id} key={d.id}>
                Main display {d.number}
              </option>
            ))}
          </select>
          {targets.length > 1 && (
            <select
              aria-label="Controlled display"
              value={target?.id ?? ""}
              onChange={(e) => {
                setTargetId(e.target.value);
                setDraft([]);
                setView(null);
              }}
            >
              {targets.map((d) => (
                <option key={d.id} value={d.id}>
                  Controlled display {d.number}
                </option>
              ))}
            </select>
          )}
          <div className="direction-buttons">
            {(
              [
                { side: "left", icon: <ArrowLeft size={17} /> },
                { side: "top", icon: <ArrowUp size={17} /> },
                { side: "bottom", icon: <ArrowDown size={17} /> },
                { side: "right", icon: <ArrowRight size={17} /> },
              ] as const
            ).map((d) => (
              <button
                key={d.side}
                aria-label={`Place controlled screen ${d.side}`}
                title={d.side}
                onClick={() => beside(d.side)}
              >
                {d.icon}
              </button>
            ))}
          </div>
          <button
            onClick={() => {
              setDraft(state.placements);
              setView(null);
            }}
          >
            Reset
          </button>
          <button
            className="primary"
            disabled={!!invalid}
            onClick={() => void run("save_layout", { placements })}
          >
            Save arrangement
          </button>
        </section>
      )}
      {canEdit && (
        <p className={`layout-message ${invalid ? "" : "success"}`}>
          {invalid ?? "Shared edge ready. Save to enable switching."}
        </p>
      )}
      <div className="display-inventory">
        {state.displays.map((d) => (
          <div key={`${d.deviceId}-${d.id}`}>
            <Monitor size={19} />
            <span>
              <strong>{d.name}</strong>
              <small>
                {d.pixelWidth} × {d.pixelHeight} · {Math.round(d.scale * 100)}%
                scale · {d.rotation}° · {d.online ? "Online" : "Offline"}
              </small>
            </span>
          </div>
        ))}
      </div>
      <div className="info-note">
        <ShieldCheck size={17} />
        <span>
          Emergency return: <kbd>Ctrl</kbd> + <kbd>Alt</kbd> + <kbd>Shift</kbd>{" "}
          + <kbd>{state.settings.emergencyKey === 1 ? "Esc" : "F12"}</kbd>.
          Works locally even if the network drops.
        </span>
      </div>
    </>
  );
}
