import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { Badge } from "./Badge";

describe("Badge", () => {
  it("renders its content", () => {
    render(<Badge tone="ok">Healthy</Badge>);
    expect(screen.getByText("Healthy")).toBeInTheDocument();
  });

  it("applies the tone classes", () => {
    render(<Badge tone="danger">Critical</Badge>);
    expect(screen.getByText("Critical").className).toContain("text-danger");
  });
});
