import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { UnitConverterPanel } from "../UnitConverterPanel";

const writeText = vi.fn().mockResolvedValue(undefined);

beforeEach(() => {
  writeText.mockClear();
  Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText } });
});

const valueInput = () => screen.getByRole("spinbutton", { name: "Value" });
const result = () => screen.getByTestId("conversion-result");

describe("UnitConverterPanel — category conversions", () => {
  it.each([
    ["Length", "1000 m"],
    ["Mass", "1000 kg"],
    ["Temperature", "33.8 °F"],
    ["Digital Storage", "0.125 B"],
    ["Speed", "3.6 km/h"],
    ["Area", "1000000 m²"],
    ["Volume", "1000 L"],
    ["Pressure", "0.001 kPa"],
    ["Energy", "0.001 kJ"],
    ["Angle", "0.01745329252 rad"],
    ["Time", "0.001 μs"],
    ["Frequency", "0.001 kHz"],
  ])("Given %s, then its default unit pair converts accurately", (category, expected) => {
    render(<UnitConverterPanel />);
    fireEvent.click(screen.getByRole("button", { name: category }));

    expect(result()).toHaveTextContent(`= ${expected}`);
  });

  it("Given Celsius, then affine temperature units convert at known reference points", () => {
    render(<UnitConverterPanel />);
    fireEvent.click(screen.getByRole("button", { name: "Temperature" }));
    fireEvent.change(valueInput(), { target: { value: "-40" } });
    expect(result()).toHaveTextContent("= -40 °F");

    fireEvent.change(screen.getByRole("combobox", { name: "To unit" }), { target: { value: "2" } });
    fireEvent.change(valueInput(), { target: { value: "0" } });
    expect(result()).toHaveTextContent("= 273.15 K");
  });
});

describe("UnitConverterPanel — interaction workflows", () => {
  it("Given selected source and target units, then changing either unit updates the result and table markers", () => {
    render(<UnitConverterPanel />);
    fireEvent.change(screen.getByRole("combobox", { name: "From unit" }), { target: { value: "6" } });
    fireEvent.change(screen.getByRole("combobox", { name: "To unit" }), { target: { value: "8" } });

    expect(result()).toHaveTextContent("= 5280 ft");
    expect(screen.getByText("Mile").parentElement).toHaveTextContent("FROM");
    expect(screen.getByText("Foot").parentElement).toHaveTextContent("TO");
  });

  it("Given a conversion, when swapped, then it preserves the quantity and reverses units", () => {
    render(<UnitConverterPanel />);
    fireEvent.click(screen.getByRole("button", { name: "Swap units" }));

    expect(valueInput()).toHaveValue(1000);
    expect(screen.getByRole("combobox", { name: "From unit" })).toHaveValue("1");
    expect(screen.getByRole("combobox", { name: "To unit" })).toHaveValue("0");
    expect(result()).toHaveTextContent("= 1 km");
  });

  it("Given a category search, then it filters categories and selecting a result clears the filter", () => {
    render(<UnitConverterPanel />);
    fireEvent.change(screen.getByRole("searchbox", { name: "Search categories" }), { target: { value: "press" } });

    expect(screen.getByRole("button", { name: "Pressure" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Length" })).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Pressure" }));
    expect(screen.getByRole("searchbox", { name: "Search categories" })).toHaveValue("");
    expect(screen.getByRole("button", { name: "Length" })).toBeInTheDocument();
  });

  it("Given an empty value, then it shows an empty state without copyable results", () => {
    render(<UnitConverterPanel />);
    fireEvent.change(valueInput(), { target: { value: "" } });

    expect(result()).toHaveTextContent("= — m");
    expect(screen.getByText(/enter a value above/)).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Copy conversion result" })).not.toBeInTheDocument();
    expect(screen.queryAllByRole("button", { name: /Copy .* result/ })).toHaveLength(0);
  });

  it("Given a valid result, then Copy writes the displayed numeric value", () => {
    render(<UnitConverterPanel />);
    fireEvent.click(screen.getByRole("button", { name: "Copy conversion result" }));

    expect(writeText).toHaveBeenCalledWith("1000");
  });
});
