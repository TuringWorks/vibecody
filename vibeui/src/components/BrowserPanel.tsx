import { useState, useRef, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { RefreshCw } from 'lucide-react';
import { useToast } from '../hooks/useToast';
import { Toaster } from './Toaster';
import { VisualEditOverlay } from './VisualEditOverlay';

const QUICK_LAUNCH = [
 { label: 'localhost:3000', url: 'http://localhost:3000' },
 { label: 'localhost:5173', url: 'http://localhost:5173' },
 { label: 'localhost:8080', url: 'http://localhost:8080' },
 { label: 'localhost:4000', url: 'http://localhost:4000' },
];

interface CdpTarget {
 id: string;
 title: string;
 url: string;
 type: string;
}

export function BrowserPanel() {
 const { toasts, toast, dismiss } = useToast();
 const [urlInput, setUrlInput] = useState('http://localhost:3000');
 const [iframeSrc, setIframeSrc] = useState('');
 const [history, setHistory] = useState<string[]>([]);
 const [histIdx, setHistIdx] = useState(-1);
 const iframeRef = useRef<HTMLIFrameElement>(null);
 const [inspectMode, setInspectMode] = useState(false);
 const [editMode, setEditMode] = useState(false);
 const [cdpConnected, setCdpConnected] = useState(false);
 const [cdpTargets, setCdpTargets] = useState<CdpTarget[]>([]);
 const [showCdp, setShowCdp] = useState(false);
 const [selectedElement, setSelectedElement] = useState<{
 selector: string;
 outerHTML: string;
 tagName: string;
 reactComponent: string | null;
 styles: Record<string, string>;
 parentChain?: string[];
 } | null>(null);
 const refreshTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

 // Listen for element-selected postMessages from the inspector
 useEffect(() => {
 const handler = (e: MessageEvent) => {
 if (e.data?.type === 'vibe:element-selected') {
 setSelectedElement(e.data.data);
 }
 };
 window.addEventListener('message', handler);
 return () => {
 window.removeEventListener('message', handler);
 if (refreshTimerRef.current) clearTimeout(refreshTimerRef.current);
 };
 }, []);

 const toggleInspect = () => {
 const iframe = iframeRef.current;
 if (!iframe?.contentWindow) return;

 if (!inspectMode) {
 // Only allow inspect on localhost URLs
 if (!iframeSrc.includes('localhost') && !iframeSrc.includes('127.0.0.1')) {
 toast.error('Inspect mode only works on localhost URLs');
 return;
 }
 // Inject inspector.js
 try {
 const script = document.createElement('script');
 script.src = '/inspector.js';
 iframe.contentDocument?.body.appendChild(script);
 } catch {
 toast.error('Cannot inject inspector — cross-origin restriction');
 return;
 }
 } else {
 // Deactivate inspector
 try {
 iframe.contentWindow.postMessage({ type: 'vibe:deactivate-inspector' }, '*');
 } catch { /* ignore */ }
 setSelectedElement(null);
 }
 setInspectMode(!inspectMode);
 };

 const sendToChat = () => {
 if (!selectedElement) return;
 const chain = selectedElement.parentChain?.join(' > ') || '';
 const styleStr = Object.entries(selectedElement.styles || {})
 .filter(([, v]) => v && v !== 'normal' && v !== 'rgba(0, 0, 0, 0)')
 .map(([k, v]) => `${k}=${v}`)
 .slice(0, 6)
 .join(', ');
 const context = [
 `@html-selected: <${selectedElement.tagName}> ${selectedElement.selector}`,
 chain ? `Parent chain: ${chain}` : '',
 selectedElement.reactComponent ? `React: <${selectedElement.reactComponent}>` : '',
 styleStr ? `Styles: ${styleStr}` : '',
 `HTML:\n${selectedElement.outerHTML.slice(0, 500)}`,
 ].filter(Boolean).join('\n');

 window.dispatchEvent(new CustomEvent('vibeui:inject-context', { detail: context }));
 toast.success('Element sent to Chat');
 };

 const navigate = (url: string) => {
 const target = url.startsWith('http://') || url.startsWith('https://') ? url : `https://${url}`;
 setUrlInput(target);
 setIframeSrc(target);
 setHistory((prev) => {
 const trimmed = prev.slice(0, histIdx + 1);
 const next = [...trimmed, target];
 setHistIdx(next.length - 1);
 return next;
 });
 };

 const goBack = () => {
 if (histIdx > 0) {
 const newIdx = histIdx - 1;
 setHistIdx(newIdx);
 setUrlInput(history[newIdx]);
 setIframeSrc(history[newIdx]);
 }
 };

 const goForward = () => {
 if (histIdx < history.length - 1) {
 const newIdx = histIdx + 1;
 setHistIdx(newIdx);
 setUrlInput(history[newIdx]);
 setIframeSrc(history[newIdx]);
 }
 };

 const refresh = () => {
 // Capture current URL before clearing, so the setTimeout callback
 // restores the correct URL even if the user navigated in the meantime.
 if (refreshTimerRef.current) clearTimeout(refreshTimerRef.current);
 setIframeSrc(prev => {
 if (prev) {
 const saved = prev;
 refreshTimerRef.current = setTimeout(() => setIframeSrc(saved), 50);
 }
 return '';
 });
 };

 const openExternal = async () => {
 if (!urlInput) return;
 try {
 await invoke('open_external_url', { url: urlInput });
 } catch (e) {
 toast.error(`Failed to open external URL: ${e}`);
 }
 };

 const connectCdp = async () => {
 try {
 const targets = await invoke<CdpTarget[]>('cdp_list_targets');
 setCdpTargets(targets);
 setCdpConnected(true);
 setShowCdp(true);
 toast.success(`CDP connected: ${targets.length} target(s)`);
 } catch (e) {
 toast.error(`${e}`);
 setCdpConnected(false);
 }
 };

 const cdpOpenTab = async (url: string) => {
 try {
 await invoke('cdp_open_tab', { url });
 toast.success('Tab opened in Chrome');
 connectCdp(); // refresh targets
 } catch (e) {
 toast.error(`Failed: ${e}`);
 }
 };

 return (
 <div className="panel-container">
 {/* Toolbar */}
 <div style={{
 display: 'flex', alignItems: 'center', gap: '4px',
 padding: '4px 8px', borderBottom: '1px solid var(--border-color)',
 background: 'var(--bg-secondary)', flexShrink: 0,
 }}>
 <button className="panel-btn"
 onClick={goBack}
 disabled={histIdx <= 0}
 title="Back"
 style={navBtnStyle}
 >←</button>
 <button
 onClick={goForward}
 disabled={histIdx >= history.length - 1}
 title="Forward"
 style={navBtnStyle}
 >→</button>
 <button className="panel-btn" onClick={refresh} disabled={!iframeSrc} title="Refresh" aria-label="Refresh" style={navBtnStyle}><RefreshCw size={14} /></button>

 <div style={{ width: '1px', height: '16px', background: 'var(--border-color)', margin: '0 4px' }} />

 <input
 type="text"
 value={urlInput}
 onChange={(e) => setUrlInput(e.target.value)}
 onKeyDown={(e) => { if (e.key === 'Enter') navigate(urlInput); }}
 placeholder="Enter URL…"
 style={{
 flex: 1, padding: '3px 8px', fontSize: '12px',
 background: 'var(--bg-tertiary)', border: '1px solid var(--border-color)',
 color: 'var(--text-primary)', borderRadius: '4px', outline: 'none',
 }}
 />
 <button
 onClick={() => navigate(urlInput)}
 style={{ ...navBtnStyle, padding: '3px 12px', fontSize: '12px' }}
 >Go</button>
 <button className="panel-btn"
 onClick={openExternal}
 title="Open in system browser"
 style={{ ...navBtnStyle, fontSize: '11px', padding: '3px 8px' }}
 >↗</button>

 <div style={{ width: '1px', height: '16px', background: 'var(--border-color)', margin: '0 4px' }} />
 <button className="panel-btn"
 onClick={toggleInspect}
 disabled={!iframeSrc}
 title={inspectMode ? "Disable Inspect" : "Enable Inspect"}
 style={{
 ...navBtnStyle,
 background: inspectMode ? 'color-mix(in srgb, var(--accent-blue) 20%, transparent)' : 'none',
 borderColor: inspectMode ? 'var(--accent-color)' : 'var(--border-color)',
 color: inspectMode ? 'var(--accent-color)' : 'var(--text-primary)',
 }}
 ></button>
 <button className="panel-btn"
 onClick={connectCdp}
 title="Connect Chrome DevTools Protocol"
 style={{
 ...navBtnStyle,
 background: cdpConnected ? 'rgba(34,197,94,0.2)' : 'none',
 borderColor: cdpConnected ? 'var(--success-color)' : 'var(--border-color)',
 color: cdpConnected ? 'var(--success-color)' : 'var(--text-primary)',
 fontSize: '11px', padding: '3px 8px',
 }}
 >CDP</button>
 <button
 onClick={() => setShowCdp(!showCdp)}
 disabled={!cdpConnected}
 title="Toggle CDP targets panel"
 style={{ ...navBtnStyle, fontSize: '11px', padding: '3px 8px' }}
 >{showCdp ? 'Hide' : 'Show'} Targets</button>
 </div>

 {/* Quick-launch chips */}
 <div style={{
 display: 'flex', gap: '8px', padding: '4px 8px',
 borderBottom: '1px solid var(--border-color)',
 background: 'var(--bg-secondary)', flexShrink: 0,
 }}>
 {QUICK_LAUNCH.map(({ label, url }) => (
 <button
 key={url}
 onClick={() => navigate(url)}
 style={{
 padding: '2px 8px', fontSize: '11px',
 background: iframeSrc === url ? 'var(--accent-color)' : 'var(--bg-tertiary)',
 color: iframeSrc === url ? 'var(--text-primary)' : 'var(--text-secondary)',
 border: '1px solid var(--border-color)', borderRadius: '10px',
 cursor: 'pointer',
 }}
 >{label}</button>
 ))}
 </div>

 {/* CDP targets panel */}
 {showCdp && cdpConnected && cdpTargets.length > 0 && (
 <div style={{
 padding: '8px 8px', borderBottom: '1px solid var(--border-color)',
 background: 'var(--bg-secondary)', flexShrink: 0, maxHeight: 120, overflowY: 'auto',
 }}>
 <div style={{ fontSize: "var(--font-size-sm)", color: 'var(--text-secondary)', marginBottom: 4 }}>
 Chrome DevTools Targets ({cdpTargets.length})
 </div>
 {cdpTargets.filter((t: CdpTarget) => t.type === 'page').map((t: CdpTarget) => (
 <div role="button" tabIndex={0} key={t.id} style={{
 display: 'flex', gap: 6, alignItems: 'center', padding: '2px 4px',
 fontSize: "var(--font-size-sm)", borderRadius: 3, cursor: 'pointer',
 }} onClick={() => { setUrlInput(t.url); navigate(t.url); }}>
 <span style={{
 padding: '0 4px', borderRadius: 2, fontSize: 9,
 background: 'rgba(34,197,94,0.15)', color: 'var(--success-color)',
 }}>PAGE</span>
 <span style={{ color: 'var(--text-primary)', fontWeight: 500 }}>{t.title.slice(0, 40)}</span>
 <span style={{ color: 'var(--text-secondary)', fontFamily: 'var(--font-mono)', fontSize: "var(--font-size-xs)" }}>
 {t.url.slice(0, 60)}
 </span>
 <button onClick={(e) => { e.stopPropagation(); cdpOpenTab(t.url); }}
 style={{ padding: '1px 4px', fontSize: 9, background: 'none', border: '1px solid var(--border-color)', color: 'var(--text-secondary)', borderRadius: 2, cursor: 'pointer' }}
 >Open</button>
 </div>
 ))}
 </div>
 )}

 {/* Webview / iframe */}
 <div style={{ flex: 1, position: 'relative', overflow: 'hidden' }}>
 {iframeSrc ? (
 <iframe
 ref={iframeRef}
 src={iframeSrc}
 sandbox="allow-scripts allow-same-origin allow-forms allow-modals"
 style={{ width: '100%', height: '100%', border: 'none' }}
 title="Browser preview"
 />
 ) : (
 <div style={{
 display: 'flex', alignItems: 'center', justifyContent: 'center',
 flex: 1, minHeight: 0, flexDirection: 'column', gap: '12px',
 color: 'var(--text-secondary)', fontSize: '13px',
 }}>
 <div style={{ fontSize: '32px' }}></div>
 <div>Enter a URL or click a quick-launch chip to preview</div>
 </div>
 )}
 {selectedElement && inspectMode && !editMode && (
 <div style={{
 position: 'absolute', bottom: 0, left: 0, right: 0,
 background: 'var(--bg-secondary)', borderTop: '1px solid var(--border-color)',
 padding: '8px 12px', fontSize: '12px', fontFamily: 'var(--font-mono)',
 maxHeight: '180px', overflowY: 'auto',
 }}>
 <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '4px' }}>
 <span style={{ fontWeight: 'bold', color: 'var(--accent-color)' }}>
 &lt;{selectedElement.tagName}&gt;
 {selectedElement.reactComponent && (
 <span style={{ color: 'var(--text-secondary)', marginLeft: '8px' }}>
 React: &lt;{selectedElement.reactComponent}&gt;
 </span>
 )}
 </span>
 <div style={{ display: 'flex', gap: '8px' }}>
 <button
 onClick={() => setEditMode(true)}
 style={{
 background: 'var(--accent-color)', color: 'var(--text-primary)',
 border: 'none', borderRadius: '4px', padding: '3px 12px',
 cursor: 'pointer', fontSize: '11px', fontWeight: 600,
 }}
 >Edit</button>
 <button className="panel-btn"
 onClick={sendToChat}
 style={{
 background: 'var(--accent-color)', color: 'var(--text-primary)',
 border: 'none', borderRadius: '4px', padding: '3px 12px',
 cursor: 'pointer', fontSize: '11px',
 }}
 >Send to Chat</button>
 </div>
 </div>
 <div style={{ color: 'var(--text-secondary)', marginBottom: '2px' }}>
 {selectedElement.selector}
 </div>
 {selectedElement.parentChain && selectedElement.parentChain.length > 0 && (
 <div style={{ color: 'var(--text-secondary)', marginBottom: '2px' }}>
 Chain: {selectedElement.parentChain.join(' > ')}
 </div>
 )}
 <pre style={{
 margin: '4px 0 0 0', padding: '8px', background: 'var(--bg-tertiary)',
 borderRadius: '3px', fontSize: '11px', whiteSpace: 'pre-wrap',
 maxHeight: '80px', overflowY: 'auto', color: 'var(--text-primary)',
 }}>
 {selectedElement.outerHTML.slice(0, 500)}
 </pre>
 </div>
 )}
 {selectedElement && inspectMode && editMode && (
 <VisualEditOverlay
 element={selectedElement}
 onClose={() => setEditMode(false)}
 onApply={(desc) => {
 window.dispatchEvent(new CustomEvent('vibeui:inject-context', { detail: desc }));
 toast.success('Edit applied');
 }}
 />
 )}
 </div>
 <Toaster toasts={toasts} onDismiss={dismiss} />
 </div>
 );
}

const navBtnStyle: React.CSSProperties = {
 background: 'none',
 border: '1px solid var(--border-color)',
 color: 'var(--text-primary)',
 borderRadius: '4px',
 cursor: 'pointer',
 padding: '3px 7px',
 fontSize: '14px',
 lineHeight: 1,
};
