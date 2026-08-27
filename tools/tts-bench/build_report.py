import json, pathlib
d = json.loads(pathlib.Path("out/clips.json").read_text())

BARS = [
    ("Apple Samantha", "compact · AVSpeechSynthesizer", 21, "0.019", "ship", ""),
    ("Apple enhanced / premium", "neural · free download", None, "—", "gap", "not installed on this Mac"),
    ("Kokoro-82M", "MLX", 471, "0.106", "ok", ""),
    ("Kokoro-82M", "ONNX fp16 · CPU", 826, "0.262", "mid", ""),
    ("Kokoro-82M", "ONNX fp32 · CPU", 869, "0.293", "mid", ""),
    ("Kokoro-82M", "ONNX fp16 · CoreML", 930, "0.297", "mid", "CoreML slower than CPU"),
    ("Kokoro-82M", "ONNX int8 · CPU", 1927, "0.614", "bad", "quantised, and worse"),
    ("Kokoro-82M", "ONNX int8 · CoreML", 2181, "0.695", "bad", ""),
]
MAXMS = 2181

def bar_rows():
    import math
    rows = []
    for name, sub, ms, rtf, kind, note in BARS:
        if ms is None:
            rows.append(f'''<tr class="r-gap">
  <th scope="row"><span class="e-name">{name}</span><span class="e-sub">{sub}</span></th>
  <td class="c-bar"><div class="bar bar-gap"><span>{note}</span></div></td>
  <td class="c-ms">—</td><td class="c-rtf">—</td></tr>''')
            continue
        # Log scale: 21 ms and 2181 ms cannot share a linear axis without the
        # shipping engine becoming an invisible sliver.
        w = (math.log10(ms) - math.log10(15)) / (math.log10(MAXMS * 1.15) - math.log10(15)) * 100
        n = f'<span class="b-note">{note}</span>' if note else ""
        rows.append(f'''<tr>
  <th scope="row"><span class="e-name">{name}</span><span class="e-sub">{sub}</span></th>
  <td class="c-bar"><div class="bar b-{kind}" style="width:{w:.1f}%"></div>{n}</td>
  <td class="c-ms">{ms:,}</td><td class="c-rtf">{rtf}</td></tr>''')
    return "\n".join(rows)

eng_head = "".join(
    f'<th scope="col"><span class="e-name">{e["name"]}</span><span class="e-sub">{e["sub"]}</span></th>'
    for e in d["engines"])

listen_rows = []
for i, s in enumerate(d["sentences"]):
    cells = "".join(
        f'<td><button class="play" data-src="{i}-{j}" aria-label="Play {e["name"]}, sentence {i+1}">'
        f'<svg viewBox="0 0 16 16" aria-hidden="true"><path class="ico-play" d="M4 2.5v11l9-5.5z"/>'
        f'<rect class="ico-stop" x="4" y="3" width="3" height="10" rx="1"/>'
        f'<rect class="ico-stop" x="9" y="3" width="3" height="10" rx="1"/></svg></button></td>'
        for j, e in enumerate(d["engines"]))
    listen_rows.append(f'<tr><th scope="row" class="c-sent">{s}</th>{cells}</tr>')

html = f'''<title>Which Voice, and What It Costs</title>
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Archivo:wght@500;600;700&family=Source+Serif+4:opsz,wght@8..60,400;8..60,600&family=JetBrains+Mono:wght@400;600&display=swap">
<style>
:root {{
  --ground:#EDF0F3; --surface:#FFFFFF; --line:#D3D9E0; --line-soft:#E4E8ED;
  --ink:#131820; --ink-2:#4C5663; --ink-3:#78828F;
  --signal:#A96A12; --signal-soft:#F0E0C6;
  --fast:#0E6E60; --mid:#8A7430; --slow:#A63A2B;
  --shadow:0 1px 2px rgba(19,24,32,.06), 0 8px 24px rgba(19,24,32,.05);
}}
@media (prefers-color-scheme: dark) {{
  :root:not([data-theme="light"]) {{
    --ground:#0E1218; --surface:#161C24; --line:#2A323D; --line-soft:#212832;
    --ink:#E6EAEF; --ink-2:#A3AEBC; --ink-3:#77828F;
    --signal:#E2A247; --signal-soft:#3A2E1B;
    --fast:#46C3AC; --mid:#C9A94E; --slow:#E4705E;
    --shadow:0 1px 2px rgba(0,0,0,.4), 0 8px 28px rgba(0,0,0,.35);
  }}
}}
:root[data-theme="dark"] {{
  --ground:#0E1218; --surface:#161C24; --line:#2A323D; --line-soft:#212832;
  --ink:#E6EAEF; --ink-2:#A3AEBC; --ink-3:#77828F;
  --signal:#E2A247; --signal-soft:#3A2E1B;
  --fast:#46C3AC; --mid:#C9A94E; --slow:#E4705E;
  --shadow:0 1px 2px rgba(0,0,0,.4), 0 8px 28px rgba(0,0,0,.35);
}}
*{{box-sizing:border-box}}
body{{margin:0;background:var(--ground);color:var(--ink);
  font-family:"Source Serif 4",Georgia,serif;font-size:17px;line-height:1.62;
  -webkit-font-smoothing:antialiased}}
.wrap{{max-width:1000px;margin:0 auto;padding:0 24px 96px}}
.col{{max-width:66ch}}
h1,h2,h3,.lbl,th,.e-name,.e-sub,button{{font-family:Archivo,"Helvetica Neue",Arial,sans-serif}}
h1{{font-size:clamp(2.1rem,5vw,3.1rem);line-height:1.04;font-weight:700;letter-spacing:-.022em;
  text-wrap:balance;margin:0 0 18px}}
h2{{font-size:1.42rem;font-weight:600;letter-spacing:-.012em;margin:64px 0 6px;text-wrap:balance}}
h3{{font-size:1.02rem;font-weight:600;margin:30px 0 4px}}
p{{margin:0 0 16px}}
.lbl{{font-size:.68rem;font-weight:600;letter-spacing:.13em;text-transform:uppercase;color:var(--signal)}}
.sub{{color:var(--ink-2);font-size:1.06rem}}
.mono{{font-family:"JetBrains Mono",ui-monospace,Menlo,monospace;font-variant-numeric:tabular-nums}}
header{{padding:72px 0 8px;border-bottom:1px solid var(--line);margin-bottom:8px}}
.meta{{display:flex;flex-wrap:wrap;gap:8px 22px;margin:22px 0 30px;
  font-family:"JetBrains Mono",monospace;font-size:.76rem;color:var(--ink-3)}}

/* The finding that reframes everything gets the one loud element on the page. */
.verdict{{background:var(--surface);border:1px solid var(--line);border-left:3px solid var(--signal);
  border-radius:3px;padding:22px 26px;margin:34px 0 8px;box-shadow:var(--shadow)}}
.verdict p{{margin:0}}
.verdict p + p{{margin-top:12px}}
.big{{font-family:Archivo,sans-serif;font-size:1.24rem;font-weight:600;line-height:1.4;letter-spacing:-.01em}}

.panel{{background:var(--surface);border:1px solid var(--line);border-radius:4px;
  box-shadow:var(--shadow);margin:26px 0;overflow-x:auto}}
table{{width:100%;border-collapse:collapse;font-size:.92rem}}
th,td{{text-align:left;padding:11px 14px;border-bottom:1px solid var(--line-soft)}}
tr:last-child th,tr:last-child td{{border-bottom:0}}
thead th{{font-size:.68rem;letter-spacing:.1em;text-transform:uppercase;color:var(--ink-3);
  font-weight:600;border-bottom:1px solid var(--line)}}
.e-name{{display:block;font-size:.9rem;font-weight:600;letter-spacing:-.005em}}
.e-sub{{display:block;font-size:.72rem;color:var(--ink-3);font-weight:500;margin-top:1px}}

.c-bar{{width:52%;min-width:200px;position:relative}}
.bar{{height:15px;border-radius:2px;background:var(--mid);min-width:3px;display:block}}
.b-ship{{background:var(--fast)}} .b-ok{{background:var(--signal)}}
.b-mid{{background:var(--mid)}} .b-bad{{background:var(--slow)}}
.bar-gap{{width:100%;background:transparent;border:1px dashed var(--line);height:15px;
  display:flex;align-items:center;padding-left:8px}}
.bar-gap span,.b-note{{font-family:Archivo,sans-serif;font-size:.68rem;color:var(--ink-3);
  letter-spacing:.01em;white-space:nowrap}}
.b-note{{margin-left:9px;display:inline-block;vertical-align:2px}}
.c-ms,.c-rtf{{font-family:"JetBrains Mono",monospace;font-variant-numeric:tabular-nums;
  text-align:right;white-space:nowrap;font-size:.85rem}}
.c-ms{{font-weight:600}} .c-rtf{{color:var(--ink-3)}}
.r-gap th .e-name{{color:var(--ink-3)}}

.c-sent{{font-family:"Source Serif 4",serif;font-weight:400;font-size:.94rem;
  color:var(--ink-2);max-width:340px}}
.play{{width:32px;height:32px;border-radius:50%;border:1px solid var(--line);
  background:var(--ground);color:var(--ink-2);cursor:pointer;display:grid;place-items:center;
  transition:background .12s,color .12s,border-color .12s}}
.play svg{{width:14px;height:14px;fill:currentColor}}
.play .ico-stop{{display:none}}
.play:hover{{border-color:var(--signal);color:var(--signal)}}
.play:focus-visible{{outline:2px solid var(--signal);outline-offset:2px}}
.play[aria-pressed="true"]{{background:var(--signal);border-color:var(--signal);color:var(--surface)}}
.play[aria-pressed="true"] .ico-play{{display:none}}
.play[aria-pressed="true"] .ico-stop{{display:block}}

.finds{{display:grid;gap:1px;background:var(--line);border:1px solid var(--line);
  border-radius:4px;overflow:hidden;margin:26px 0}}
@media (min-width:720px){{.finds{{grid-template-columns:repeat(3,1fr)}}}}
.find{{background:var(--surface);padding:20px 22px}}
.find .n{{font-family:"JetBrains Mono",monospace;font-size:.72rem;color:var(--signal);
  font-weight:600;display:block;margin-bottom:7px}}
.find h3{{margin:0 0 6px;font-size:.98rem}}
.find p{{margin:0;font-size:.9rem;line-height:1.55;color:var(--ink-2)}}

.trap{{border-left:2px solid var(--slow);padding:2px 0 2px 18px;margin:20px 0}}
.trap h3{{margin:0 0 4px;color:var(--slow);font-size:.95rem}}
.trap p{{margin:0;font-size:.95rem;color:var(--ink-2)}}
code{{font-family:"JetBrains Mono",monospace;font-size:.86em;background:var(--signal-soft);
  padding:1px 5px;border-radius:3px}}
footer{{margin-top:72px;padding-top:22px;border-top:1px solid var(--line);
  font-family:"JetBrains Mono",monospace;font-size:.74rem;color:var(--ink-3)}}
@media (prefers-reduced-motion:reduce){{*{{transition:none!important}}}}
</style>

<div class="wrap">
<header>
  <span class="lbl">Speech synthesis · bench · 27 Aug 2026</span>
  <h1>Which voice, and what it costs</h1>
  <p class="sub col">Five sentences through every candidate engine, measured on one
  M-series Mac. The complaint was that the assistant sounds mechanical. The
  measurement says the engine was never the problem.</p>
  <div class="meta">
    <span>macOS 26.6.2</span><span>onnxruntime 1.29.0</span>
    <span>kokoro-82M v1.0</span><span>5 sentences · warmed</span>
  </div>
</header>

<div class="verdict col">
  <p class="big">Your Mac has 180 voices installed and every one of them is the
  compact tier.</p>
  <p>Zero enhanced, zero premium, zero downloaded voice assets — the asset
  directories are empty. Apple's neural voices are free, sound dramatically
  better, and cost nothing at synthesis time. They are a separate download that
  has never been made on this machine, which is why the cheapest row in the
  table below is the one row with no number in it.</p>
</div>

<h2>First audio</h2>
<p class="col">The number a listener actually feels: how long after the sentence
is decided before the first sample can play. The axis is logarithmic, because
21 ms and 2,181 ms do not share a linear one.</p>

<div class="panel">
<table>
  <thead><tr><th scope="col">Engine</th><th scope="col">First audio</th>
  <th scope="col" class="c-ms">ms</th><th scope="col" class="c-rtf">RTF</th></tr></thead>
  <tbody>
{bar_rows()}
  </tbody>
</table>
</div>
<p class="col"><strong>RTF</strong> is synthesis seconds per second of audio. Under
1.0 the engine keeps ahead of playback, so only the <em>first</em> sentence of a
reply is latency anyone waits through — every engine here clears that bar. The
first column is what changes the feel of a conversation.</p>

<h2>Hear it</h2>
<p class="col">Latency is measurable and quality is not. These are the actual
files the bench wrote, unedited. The question the numbers cannot answer is
whether the fast one is good enough.</p>

<div class="panel">
<table>
  <thead><tr><th scope="col">Sentence</th>{eng_head}</tr></thead>
  <tbody>
{chr(10).join(listen_rows)}
  </tbody>
</table>
</div>

<h2>Three results that cut against expectation</h2>
<p class="col">Each of these would have been guessed the other way, which is the
entire reason for measuring rather than estimating.</p>
<div class="finds">
  <div class="find"><span class="n">CoreML</span>
    <h3>The accelerator is slower</h3>
    <p>At every precision, and it adds 1.6–2.7 s of session load. Asking for
    CoreML is not the same as getting it — the graph partitions badly and falls
    back mid-flight.</p></div>
  <div class="find"><span class="n">int8</span>
    <h3>Quantisation made it worse</h3>
    <p>2.3× slower than fp16, not faster. An 82M-parameter model is not
    compute-bound in the way quantisation helps, so the smaller file buys
    nothing and costs a great deal.</p></div>
  <div class="find"><span class="n">MLX</span>
    <h3>The runtime was the variable</h3>
    <p>1.75× faster than the best ONNX build, on the same weights and the same
    machine. The published 0.08 RTF is reachable — just not through ONNX
    Runtime.</p></div>
</div>

<h2>Two instrument bugs</h2>
<p class="col">Both were caught by disbelieving a number, and both were wrong in
the direction that would have changed the decision.</p>

<div class="trap col">
  <h3>Apple measured 245 ms</h3>
  <p>The bench allocated a fresh <code>AVSpeechSynthesizer</code> per utterance.
  The shipping sidecar's own comments say that costs ~185 ms every time, which
  is exactly why it keeps one alive. Sharing it gave 21 ms — a 12× error,
  every bit of it flattering to the neural alternatives.</p>
</div>
<div class="trap col">
  <h3>The first run reported zero rows, not a failure</h3>
  <p><code>write</code> was dispatched to the main queue while the main thread
  sat blocked on the semaphore waiting for it, so the work could never be
  scheduled. An empty table is what a deadlock looks like when nothing checks
  for one.</p>
</div>

<h2>What this does not tell you</h2>
<p class="col"><strong>Apple's enhanced and premium voices are unmeasured.</strong>
They are the cheapest option on the table and the only row without a number,
because none is installed here. Install one — System Settings → Accessibility →
Spoken Content → System Voice → Manage Voices — and re-run: the bench picks up
every installed <code>com.apple.voice.*</code> voice automatically and fills the
row in.</p>
<p class="col">Nothing here measures whether a voice sounds good. RTF is silent
on the only question that was actually asked, which is why the clips are above
and not merely referenced.</p>

<footer>tools/tts-bench · ./bench.sh reproduces every number · models and venv gitignored</footer>
</div>

<script>
const CLIPS = {json.dumps(d["clips"])};
let cur = null, curBtn = null;
document.querySelectorAll(".play").forEach(btn => {{
  btn.addEventListener("click", () => {{
    const [i, j] = btn.dataset.src.split("-").map(Number);
    if (cur) {{ cur.pause(); cur = null; }}
    if (curBtn) {{ curBtn.setAttribute("aria-pressed", "false"); }}
    if (curBtn === btn) {{ curBtn = null; return; }}
    const a = new Audio(CLIPS[i][j]);
    a.addEventListener("ended", () => {{
      btn.setAttribute("aria-pressed", "false"); cur = null; curBtn = null;
    }});
    a.play();
    btn.setAttribute("aria-pressed", "true");
    cur = a; curBtn = btn;
  }});
}});
</script>
'''
pathlib.Path("out/report.html").write_text(html)
print(f"wrote out/report.html  {len(html)/1e6:.1f} MB")
