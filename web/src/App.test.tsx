import { render, screen } from "@testing-library/react";

import { App } from "./App";

describe("App", () => {
  it("renders the placeholder with a themed button", () => {
    render(<App />);
    expect(
      screen.getByRole("heading", { name: "Advent calendar" }),
    ).toBeInTheDocument();
    const button = screen.getByRole("button", { name: "Open a door" });
    expect(button).toHaveClass("bg-primary");
  });
});
