import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { CidrPanel } from "../CidrPanel";

const cidrInput = () => screen.getByPlaceholderText("192.168.1.0/24");
const infoValue = (label: string) => screen.getByText(label).parentElement;

describe("CidrPanel — calculator", () => {
  it("Given a host address, then it shows the containing subnet", () => {
    render(<CidrPanel />);

    expect(screen.getByText("192.168.1.0/24")).toBeInTheDocument();
    expect(screen.getByText("192.168.1.255")).toBeInTheDocument();
    expect(screen.getByText("192.168.1.1")).toBeInTheDocument();
    expect(screen.getByText("192.168.1.254")).toBeInTheDocument();
    expect(screen.getByText("254")).toBeInTheDocument();
  });

  it.each([
    "192.168.1x.1/24",
    "192.168.1.1/24oops",
    "192.168..1/24",
    "+192.168.1.1/24",
  ])("Given malformed CIDR %s, then it rejects the entire value", (cidr) => {
    render(<CidrPanel />);

    fireEvent.change(cidrInput(), { target: { value: cidr } });

    expect(screen.getByText(/Invalid CIDR/)).toBeInTheDocument();
    expect(screen.queryByText("CIDR Notation")).not.toBeInTheDocument();
  });

  it("Given /31 and /32 networks, then it applies point-to-point host semantics", () => {
    render(<CidrPanel />);

    fireEvent.change(cidrInput(), { target: { value: "10.0.0.4/31" } });
    expect(infoValue("CIDR Notation")).toHaveTextContent("10.0.0.4/31");
    expect(infoValue("Last Host")).toHaveTextContent("10.0.0.5");
    expect(infoValue("Usable Hosts")).toHaveTextContent("2");

    fireEvent.change(cidrInput(), { target: { value: "10.0.0.4/32" } });
    expect(infoValue("CIDR Notation")).toHaveTextContent("10.0.0.4/32");
    expect(infoValue("Usable Hosts")).toHaveTextContent("1");
  });
});

describe("CidrPanel — split and reference workflows", () => {
  it("Given a /24, when split is selected, then it lists four /26 networks", () => {
    render(<CidrPanel />);

    fireEvent.click(screen.getByRole("button", { name: "Split" }));

    expect(screen.getByText(/4 subnets × 62 hosts/)).toBeInTheDocument();
    for (const cidr of ["192.168.1.0/26", "192.168.1.64/26", "192.168.1.128/26", "192.168.1.192/26"]) {
      expect(screen.getByText(cidr)).toBeInTheDocument();
    }
  });

  it("Given a narrower network than the remembered split prefix, then it chooses the next valid prefix", () => {
    render(<CidrPanel />);
    fireEvent.change(cidrInput(), { target: { value: "10.0.0.0/30" } });

    fireEvent.click(screen.getByRole("button", { name: "Split" }));

    expect(screen.getByText("Split into /", { exact: false })).toHaveTextContent("Split into /31 blocks");
    expect(screen.getByText(/2 subnets × 0 hosts/)).toBeInTheDocument();
    expect(screen.getByText("10.0.0.0/31")).toBeInTheDocument();
    expect(screen.getByText("10.0.0.2/31")).toBeInTheDocument();
  });

  it("Given a /32 host route, when split is selected, then it explains that no smaller network exists", () => {
    render(<CidrPanel />);
    fireEvent.change(cidrInput(), { target: { value: "10.0.0.7/32" } });

    fireEvent.click(screen.getByRole("button", { name: "Split" }));

    expect(screen.getByText("A /32 host route cannot be split into smaller IPv4 networks.")).toBeInTheDocument();
    expect(screen.queryByRole("slider")).not.toBeInTheDocument();
  });

  it("Given the reference tab, when a range row is selected, then it loads that range in the calculator", () => {
    render(<CidrPanel />);
    fireEvent.click(screen.getByRole("button", { name: "Reference" }));

    fireEvent.click(screen.getByText("Class A Private (RFC 1918)"));

    expect(cidrInput()).toHaveValue("10.0.0.0/8");
    expect(infoValue("CIDR Notation")).toHaveTextContent("10.0.0.0/8");
  });
});
