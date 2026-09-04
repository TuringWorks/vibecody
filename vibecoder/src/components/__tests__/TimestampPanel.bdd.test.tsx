import { fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { TimestampPanel } from "../TimestampPanel";

function valueFor(label: string): HTMLElement {
  return screen.getByText(label).nextElementSibling as HTMLElement;
}

function cardValue(label: string): Element | null {
  return screen.getByText(label).previousElementSibling;
}

afterEach(() => vi.useRealTimers());

describe("TimestampPanel — conversion", () => {
  it("Given Unix epoch seconds, then it shows equivalent ISO and millisecond values", () => {
    render(<TimestampPanel />);
    fireEvent.change(screen.getByPlaceholderText("Unix timestamp or ISO date string…"), { target: { value: "0" } });

    expect(valueFor("ISO 8601")).toHaveTextContent("1970-01-01T00:00:00.000Z");
    expect(valueFor("Unix (seconds)")).toHaveTextContent("0");
    expect(valueFor("Unix (ms)")).toHaveTextContent("0");
    expect(valueFor("RFC 2822")).toHaveTextContent("Thu, 01 Jan 1970 00:00:00 GMT");
  });

  it("Given a negative millisecond timestamp, then unit detection preserves pre-epoch dates", () => {
    render(<TimestampPanel />);
    fireEvent.change(screen.getByPlaceholderText("Unix timestamp or ISO date string…"), { target: { value: "-2208988800000" } });

    expect(valueFor("ISO 8601")).toHaveTextContent("1900-01-01T00:00:00.000Z");
    expect(screen.queryByText(/Could not parse input/)).not.toBeInTheDocument();
  });

  it("Given malformed input, then it reports the accepted timestamp formats", () => {
    render(<TimestampPanel />);
    fireEvent.change(screen.getByPlaceholderText("Unix timestamp or ISO date string…"), { target: { value: "not-a-date" } });

    expect(screen.getByText(/Could not parse input/)).toBeInTheDocument();
    expect(screen.queryByText("ISO 8601")).not.toBeInTheDocument();
  });

  it("Given an ISO instant, when timezone changes, then locale output uses that timezone", () => {
    render(<TimestampPanel />);
    fireEvent.change(screen.getByPlaceholderText("Unix timestamp or ISO date string…"), { target: { value: "2025-01-01T00:00:00Z" } });
    fireEvent.change(screen.getByRole("combobox"), { target: { value: "America/Los_Angeles" } });

    expect(valueFor("Date only")).toHaveTextContent("December 31, 2024");
    expect(valueFor("Time only")).toHaveTextContent("4:00:00 PM");
  });

  it("Given an instant crossing a calendar boundary, then ISO week uses the selected timezone", () => {
    render(<TimestampPanel />);
    fireEvent.change(screen.getByPlaceholderText("Unix timestamp or ISO date string…"), { target: { value: "2021-01-04T00:30:00Z" } });
    fireEvent.change(screen.getByRole("combobox"), { target: { value: "America/Los_Angeles" } });
    expect(valueFor("Week of year")).toHaveTextContent("W53");

    fireEvent.change(screen.getByRole("combobox"), { target: { value: "Asia/Tokyo" } });
    expect(valueFor("Week of year")).toHaveTextContent("W01");
  });
});

describe("TimestampPanel — duration and relative dates", () => {
  it("Given end-of-month dates, then the calendar breakdown does not double-count days", () => {
    render(<TimestampPanel />);
    fireEvent.click(screen.getByRole("button", { name: "Duration" }));
    fireEvent.change(screen.getByLabelText("START"), { target: { value: "2025-01-31T00:00" } });
    fireEvent.change(screen.getByLabelText("END"), { target: { value: "2025-03-01T00:00" } });

    expect(cardValue("Years")).toHaveTextContent("0");
    expect(cardValue("Months")).toHaveTextContent("1");
    expect(cardValue("Days")).toHaveTextContent("1");
    expect(valueFor("Total days")).toHaveTextContent("29");
  });

  it("Given reversed endpoints, then duration remains positive and symmetric", () => {
    render(<TimestampPanel />);
    fireEvent.click(screen.getByRole("button", { name: "Duration" }));
    fireEvent.change(screen.getByLabelText("START"), { target: { value: "2025-01-03T12:30" } });
    fireEvent.change(screen.getByLabelText("END"), { target: { value: "2025-01-01T10:00" } });

    expect(valueFor("Total hours")).toHaveTextContent("50");
    expect(cardValue("Days")).toHaveTextContent("2");
    expect(cardValue("Hours")).toHaveTextContent("2");
    expect(cardValue("Minutes")).toHaveTextContent("30");
  });

  it("Given a future base date, then it reports relative time and calendar offsets", () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2030-01-01T12:00:00Z"));
    render(<TimestampPanel />);
    fireEvent.click(screen.getByRole("button", { name: "Relative" }));
    fireEvent.change(screen.getByLabelText("Base date:"), { target: { value: "2030-01-03T12:00" } });

    expect(screen.getByText("in 2 days")).toBeInTheDocument();
    expect(screen.getByText("+1 day").parentElement).toHaveTextContent("Jan 4, 2030");
    expect(screen.getByText("-1 day").parentElement).toHaveTextContent("Jan 2, 2030");
  });

  it("Given the formats tab, then it exposes common ecosystem references", () => {
    render(<TimestampPanel />);
    fireEvent.click(screen.getByRole("button", { name: "Formats" }));

    expect(screen.getByText("strftime (C/Python/Ruby/Go)")).toBeInTheDocument();
    expect(screen.getByText("Java / Kotlin (DateTimeFormatter)")).toBeInTheDocument();
    expect(screen.getByText(".NET (C#)")).toBeInTheDocument();
  });
});
