import type { ReactNode } from "react";
import { Badge } from "./Prose";

export function OpCard({
  name,
  tone,
  toneLabel,
  children,
}: {
  name: string;
  tone: "blue" | "green" | "red";
  toneLabel: string;
  children: ReactNode;
}) {
  return (
    <div className="op-card">
      <div className="op-card-head">
        <span className="op-name">{name}</span>
        <Badge tone={tone}>{toneLabel}</Badge>
      </div>
      <div className="op-card-body">{children}</div>
    </div>
  );
}
