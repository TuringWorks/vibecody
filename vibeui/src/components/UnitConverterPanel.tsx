import { useState, useMemo } from "react";

type UnitDef = { label: string; symbol: string; toBase: (v: number) => number; fromBase: (v: number) => number };
type Category = { name: string; icon: string; units: UnitDef[] };

const lin = (factor: number): Pick<UnitDef, "toBase" | "fromBase"> => ({
 toBase: (v) => v * factor,
 fromBase: (v) => v / factor,
});

const CATEGORIES: Category[] = [
 {
 name: "Length", icon: "",
 units: [
 { label: "Kilometre", symbol: "km", ...lin(1000) },
 { label: "Metre", symbol: "m", ...lin(1) },
 { label: "Centimetre", symbol: "cm", ...lin(0.01) },
 { label: "Millimetre", symbol: "mm", ...lin(0.001) },
 { label: "Micrometre", symbol: "μm", ...lin(1e-6) },
 { label: "Nanometre", symbol: "nm", ...lin(1e-9) },
 { label: "Mile", symbol: "mi", ...lin(1609.344) },
 { label: "Yard", symbol: "yd", ...lin(0.9144) },
 { label: "Foot", symbol: "ft", ...lin(0.3048) },
 { label: "Inch", symbol: "in", ...lin(0.0254) },
 { label: "Nautical mile", symbol: "nmi", ...lin(1852) },
 { label: "Light year", symbol: "ly", ...lin(9.461e15) },
 { label: "Astronomical unit", symbol: "AU", ...lin(1.496e11) },
 ],
 },
 {
 name: "Mass", icon: "",
 units: [
 { label: "Tonne", symbol: "t", ...lin(1e6) },
 { label: "Kilogram", symbol: "kg", ...lin(1000) },
 { label: "Gram", symbol: "g", ...lin(1) },
 { label: "Milligram", symbol: "mg", ...lin(0.001) },
 { label: "Microgram", symbol: "μg", ...lin(1e-6) },
 { label: "Pound", symbol: "lb", ...lin(453.592) },
 { label: "Ounce", symbol: "oz", ...lin(28.3495) },
 { label: "Stone", symbol: "st", ...lin(6350.29) },
 { label: "Short ton (US)", symbol: "ton", ...lin(907185) },
 ],
 },
 {
 name: "Temperature", icon: "temp",
 units: [
 {
 label: "Celsius", symbol: "°C",
 toBase: (v) => v,
 fromBase: (v) => v,
 },
 {
 label: "Fahrenheit", symbol: "°F",
 toBase: (v) => (v - 32) * 5 / 9,
 fromBase: (v) => v * 9 / 5 + 32,
 },
 {
 label: "Kelvin", symbol: "K",
 toBase: (v) => v - 273.15,
 fromBase: (v) => v + 273.15,
 },
 {
 label: "Rankine", symbol: "°R",
 toBase: (v) => (v - 491.67) * 5 / 9,
 fromBase: (v) => (v + 273.15) * 9 / 5,
 },
 ],
 },
 {
 name: "Digital Storage", icon: "",
 units: [
 { label: "Bit", symbol: "b", ...lin(1) },
 { label: "Byte", symbol: "B", ...lin(8) },
 { label: "Kilobit", symbol: "Kb", ...lin(1e3) },
 { label: "Kilobyte", symbol: "KB", ...lin(8e3) },
 { label: "Megabit", symbol: "Mb", ...lin(1e6) },
 { label: "Megabyte", symbol: "MB", ...lin(8e6) },
 { label: "Gigabit", symbol: "Gb", ...lin(1e9) },
 { label: "Gigabyte", symbol: "GB", ...lin(8e9) },
 { label: "Terabit", symbol: "Tb", ...lin(1e12) },
 { label: "Terabyte", symbol: "TB", ...lin(8e12) },
 { label: "Petabyte", symbol: "PB", ...lin(8e15) },
 { label: "Kibibyte", symbol: "KiB", ...lin(8 * 1024) },
 { label: "Mebibyte", symbol: "MiB", ...lin(8 * 1024 ** 2) },
 { label: "Gibibyte", symbol: "GiB", ...lin(8 * 1024 ** 3) },
 { label: "Tebibyte", symbol: "TiB", ...lin(8 * 1024 ** 4) },
 ],
 },
 {
 name: "Speed", icon: "",
 units: [
 { label: "m/s", symbol: "m/s", ...lin(1) },
 { label: "km/h", symbol: "km/h", ...lin(1 / 3.6) },
 { label: "mph", symbol: "mph", ...lin(0.44704) },
 { label: "ft/s", symbol: "ft/s", ...lin(0.3048) },
 { label: "Knot", symbol: "kn", ...lin(0.514444) },
 { label: "Mach (sea level)", symbol: "Ma", ...lin(340.29) },
 { label: "Speed of light", symbol: "c", ...lin(299792458) },
 ],
 },
 {
 name: "Area", icon: "area",
 units: [
 { label: "km²", symbol: "km²", ...lin(1e6) },
 { label: "m²", symbol: "m²", ...lin(1) },
 { label: "cm²", symbol: "cm²", ...lin(1e-4) },
 { label: "mm²", symbol: "mm²", ...lin(1e-6) },
 { label: "Hectare", symbol: "ha", ...lin(1e4) },
 { label: "Acre", symbol: "ac", ...lin(4046.86) },
 { label: "sq mile", symbol: "mi²", ...lin(2.59e6) },
 { label: "sq yard", symbol: "yd²", ...lin(0.836127) },
 { label: "sq foot", symbol: "ft²", ...lin(0.092903) },
 { label: "sq inch", symbol: "in²", ...lin(6.4516e-4) },
 ],
 },
 {
 name: "Volume", icon: "",
 units: [
 { label: "Cubic metre", symbol: "m³", ...lin(1000) },
 { label: "Litre", symbol: "L", ...lin(1) },
 { label: "Millilitre", symbol: "mL", ...lin(0.001) },
 { label: "Cubic centimetre", symbol: "cm³", ...lin(0.001) },
 { label: "Cubic foot", symbol: "ft³", ...lin(28.3168) },
 { label: "Cubic inch", symbol: "in³", ...lin(0.0163871) },
 { label: "US gallon", symbol: "gal", ...lin(3.78541) },
 { label: "US quart", symbol: "qt", ...lin(0.946353) },
 { label: "US pint", symbol: "pt", ...lin(0.473176) },
 { label: "US cup", symbol: "cup", ...lin(0.236588) },
 { label: "US fl oz", symbol: "fl oz", ...lin(0.0295735) },
 { label: "Imp gallon", symbol: "imp gal", ...lin(4.54609) },
 { label: "Tablespoon", symbol: "tbsp", ...lin(0.0147868) },
 { label: "Teaspoon", symbol: "tsp", ...lin(0.00492892) },
 ],
 },
 {
 name: "Pressure", icon: "pressure",
 units: [
 { label: "Pascal", symbol: "Pa", ...lin(1) },
 { label: "Kilopascal", symbol: "kPa", ...lin(1000) },
 { label: "Megapascal", symbol: "MPa", ...lin(1e6) },
 { label: "Bar", symbol: "bar", ...lin(1e5) },
 { label: "Millibar", symbol: "mbar", ...lin(100) },
 { label: "Atmosphere", symbol: "atm", ...lin(101325) },
 { label: "mmHg (torr)", symbol: "mmHg", ...lin(133.322) },
 { label: "psi", symbol: "psi", ...lin(6894.76) },
 { label: "kgf/cm²", symbol: "kgf/cm²", ...lin(98066.5) },
 ],
 },
 {
 name: "Energy", icon: "",
 units: [
 { label: "Joule", symbol: "J", ...lin(1) },
 { label: "Kilojoule", symbol: "kJ", ...lin(1000) },
 { label: "Megajoule", symbol: "MJ", ...lin(1e6) },
 { label: "Calorie (th)", symbol: "cal", ...lin(4.184) },
 { label: "Kilocalorie", symbol: "kcal", ...lin(4184) },
 { label: "Watt-hour", symbol: "Wh", ...lin(3600) },
 { label: "Kilowatt-hour", symbol: "kWh", ...lin(3.6e6) },
 { label: "Electronvolt", symbol: "eV", ...lin(1.602e-19) },
 { label: "BTU", symbol: "BTU", ...lin(1055.06) },
 { label: "Foot-pound", symbol: "ft·lb", ...lin(1.35582) },
 { label: "Therm", symbol: "thm", ...lin(1.055e8) },
 ],
 },
 {
 name: "Angle", icon: "",
 units: [
 { label: "Degree", symbol: "°", ...lin(Math.PI / 180) },
 { label: "Radian", symbol: "rad", ...lin(1) },
 { label: "Gradian", symbol: "grad", ...lin(Math.PI / 200) },
 { label: "Arcminute", symbol: "′", ...lin(Math.PI / 10800) },
 { label: "Arcsecond", symbol: "″", ...lin(Math.PI / 648000) },
 { label: "Turn", symbol: "turn", ...lin(2 * Math.PI) },
 ],
 },
 {
 name: "Time", icon: "time",
 units: [
 { label: "Nanosecond", symbol: "ns", ...lin(1e-9) },
 { label: "Microsecond", symbol: "μs", ...lin(1e-6) },
 { label: "Millisecond", symbol: "ms", ...lin(0.001) },
 { label: "Second", symbol: "s", ...lin(1) },
 { label: "Minute", symbol: "min", ...lin(60) },
 { label: "Hour", symbol: "h", ...lin(3600) },
 { label: "Day", symbol: "d", ...lin(86400) },
 { label: "Week", symbol: "wk", ...lin(604800) },
 { label: "Month (avg)", symbol: "mo", ...lin(2629800) },
 { label: "Year (avg)", symbol: "yr", ...lin(31557600) },
 { label: "Decade", symbol: "dec", ...lin(315576000) },
 { label: "Century", symbol: "cent", ...lin(3155760000) },
 ],
 },
 {
 name: "Frequency", icon: "freq",
 units: [
 { label: "Hertz", symbol: "Hz", ...lin(1) },
 { label: "Kilohertz", symbol: "kHz", ...lin(1e3) },
 { label: "Megahertz", symbol: "MHz", ...lin(1e6) },
 { label: "Gigahertz", symbol: "GHz", ...lin(1e9) },
 { label: "RPM", symbol: "rpm", ...lin(1 / 60) },
 { label: "rad/s", symbol: "rad/s", ...lin(1 / (2 * Math.PI)) },
 ],
 },
];

function fmt(n: number): string {
 if (!isFinite(n)) return "—";
 if (n === 0) return "0";
 const abs = Math.abs(n);
 if (abs >= 1e15 || (abs < 1e-6 && abs > 0)) return n.toExponential(6);
 if (abs >= 1000) return n.toPrecision(10).replace(/\.?0+$/, "");
 return parseFloat(n.toPrecision(10)).toString();
}

export function UnitConverterPanel() {
 const [catIdx, setCatIdx] = useState(0);
 const [fromIdx, setFromIdx] = useState(0);
 const [toIdx, setToIdx] = useState(1);
 const [inputVal, setInputVal] = useState("1");
 const [search, setSearch] = useState("");

 const cat = CATEGORIES[catIdx];
 const filteredCats = useMemo(() =>
 search ? CATEGORIES.filter(c => c.name.toLowerCase().includes(search.toLowerCase())) : CATEGORIES,
 [search]);

 const convert = (from: UnitDef, to: UnitDef, val: number) => {
 const base = from.toBase(val);
 return to.fromBase(base);
 };

 const inputNum = parseFloat(inputVal);
 const fromUnit = cat.units[fromIdx];
 const toUnit = cat.units[toIdx];
 const result = !isNaN(inputNum) ? convert(fromUnit, toUnit, inputNum) : NaN;

 // All units table
 const allResults = useMemo(() => {
 if (isNaN(inputNum)) return [];
 const base = fromUnit.toBase(inputNum);
 return cat.units.map(u => ({ unit: u, value: u.fromBase(base) }));
 }, [inputNum, fromUnit, cat]);

 const swap = () => {
 setFromIdx(toIdx);
 setToIdx(fromIdx);
 if (!isNaN(result)) setInputVal(fmt(result));
 };

 const sel: React.CSSProperties = { background: "var(--bg-tertiary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", borderRadius: 4, padding: "5px 8px", fontSize: 13 };

 return (
 <div style={{ display: "flex", height: "100%", fontSize: 13, color: "var(--text-primary)" }}>
 {/* Category sidebar */}
 <div style={{ width: 140, borderRight: "1px solid var(--border-color)", display: "flex", flexDirection: "column", overflow: "hidden", flexShrink: 0 }}>
 <input value={search} onChange={e => setSearch(e.target.value)} placeholder="Search…"
 style={{ margin: 6, padding: "4px 7px", background: "var(--bg-secondary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", borderRadius: 4, fontSize: 12 }} />
 <div style={{ flex: 1, overflowY: "auto" }}>
 {filteredCats.map((c) => {
 const realIdx = CATEGORIES.indexOf(c);
 return (
 <button key={c.name} onClick={() => { setCatIdx(realIdx); setFromIdx(0); setToIdx(1); setSearch(""); }}
 style={{ display: "flex", alignItems: "center", gap: 6, width: "100%", padding: "7px 10px", border: "none", background: catIdx === realIdx ? "rgba(var(--accent-rgb,99,102,241),0.18)" : "none", color: catIdx === realIdx ? "var(--accent-color)" : "var(--text-secondary)", textAlign: "left", cursor: "pointer", fontSize: 12, borderLeft: catIdx === realIdx ? "3px solid var(--accent-color)" : "3px solid transparent" }}>
 <span>{c.icon}</span>{c.name}
 </button>
 );
 })}
 </div>
 </div>

 {/* Main */}
 <div style={{ flex: 1, display: "flex", flexDirection: "column", overflow: "hidden" }}>
 {/* Converter card */}
 <div style={{ padding: "16px 20px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)" }}>
 <div style={{ fontSize: 14, fontWeight: 600, marginBottom: 12 }}>{cat.icon} {cat.name}</div>
 <div style={{ display: "flex", alignItems: "center", gap: 10, flexWrap: "wrap" }}>
 <input value={inputVal} onChange={e => setInputVal(e.target.value)} type="number"
 style={{ width: 140, padding: "6px 10px", background: "var(--bg-primary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", borderRadius: 4, fontSize: 14 }} />
 <select value={fromIdx} onChange={e => setFromIdx(Number(e.target.value))} style={{ ...sel, minWidth: 150 }}>
 {cat.units.map((u, i) => <option key={i} value={i}>{u.label} ({u.symbol})</option>)}
 </select>
 <button onClick={swap} title="Swap"
 style={{ padding: "5px 10px", background: "none", border: "1px solid var(--border-color)", borderRadius: 4, cursor: "pointer", color: "var(--text-primary)", fontSize: 16 }}>⇄</button>
 <select value={toIdx} onChange={e => setToIdx(Number(e.target.value))} style={{ ...sel, minWidth: 150 }}>
 {cat.units.map((u, i) => <option key={i} value={i}>{u.label} ({u.symbol})</option>)}
 </select>
 <span style={{ fontSize: 15, fontWeight: 600, color: "var(--accent-color)", minWidth: 180 }}>
 = {isNaN(result) ? "—" : fmt(result)} {toUnit.symbol}
 </span>
 {!isNaN(result) && (
 <button onClick={() => navigator.clipboard.writeText(fmt(result))}
 style={{ padding: "4px 8px", background: "none", border: "1px solid var(--border-color)", borderRadius: 4, cursor: "pointer", color: "var(--text-secondary)", fontSize: 11 }}>
 Copy
 </button>
 )}
 </div>
 </div>

 {/* All units table */}
 <div style={{ flex: 1, overflowY: "auto", padding: "10px 20px" }}>
 <div style={{ fontSize: 11, color: "var(--text-secondary)", marginBottom: 8 }}>
 All {cat.name} units — {isNaN(inputNum) ? "enter a value above" : `${inputVal} ${fromUnit.symbol}`}
 </div>
 <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 12 }}>
 <thead>
 <tr>
 {["Unit", "Symbol", "Value"].map(h => (
 <th key={h} style={{ padding: "5px 10px", textAlign: "left", borderBottom: "2px solid var(--accent-blue)", color: "var(--text-secondary)", fontSize: 11 }}>{h}</th>
 ))}
 <th style={{ width: 50 }} />
 </tr>
 </thead>
 <tbody>
 {allResults.map(({ unit, value }, i) => {
 const isFrom = cat.units[fromIdx] === unit;
 const isTo = cat.units[toIdx] === unit;
 return (
 <tr key={i}
 style={{ background: isFrom ? "rgba(var(--accent-rgb,99,102,241),0.08)" : isTo ? "rgba(100,200,100,0.08)" : i % 2 === 0 ? "transparent" : "rgba(255,255,255,0.02)" }}>
 <td style={{ padding: "4px 10px", fontWeight: isFrom || isTo ? 600 : undefined }}>
 {unit.label}
 {isFrom && <span style={{ marginLeft: 6, fontSize: 10, color: "var(--accent-color)" }}>FROM</span>}
 {isTo && <span style={{ marginLeft: 6, fontSize: 10, color: "var(--success-color)" }}>TO</span>}
 </td>
 <td style={{ padding: "4px 10px", color: "var(--text-secondary)", fontFamily: "var(--font-mono)" }}>{unit.symbol}</td>
 <td style={{ padding: "4px 10px", fontFamily: "var(--font-mono)", color: "var(--text-primary)" }}>
 {isNaN(inputNum) ? "—" : fmt(value)}
 </td>
 <td style={{ padding: "4px 6px" }}>
 <button onClick={() => navigator.clipboard.writeText(isNaN(inputNum) ? "" : fmt(value))} title="Copy"
 style={{ background: "none", border: "none", cursor: "pointer", color: "var(--text-secondary)", fontSize: 11, opacity: isNaN(inputNum) ? 0.3 : 1 }}>⎘</button>
 </td>
 </tr>
 );
 })}
 </tbody>
 </table>
 </div>
 </div>
 </div>
 );
}
