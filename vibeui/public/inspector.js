/**
 * VibeCody Visual Inspector — injected into the Browser Panel iframe.
 *
 * Highlights elements on hover, selects on click, and sends element info
 * to the parent window via postMessage.
 *
 * Parent receives: { type: "vibe:element-selected", data: SelectedElement }
 *                  { type: "vibe:element-hovered", data: SelectedElement }
 */
(function () {
    'use strict';

    if (window.__vibeInspectorActive) return;
    window.__vibeInspectorActive = true;

    // ── Overlay element ──────────────────────────────────────────────────────
    const overlay = document.createElement('div');
    overlay.id = '__vibe_overlay';
    overlay.style.cssText = [
        'position:fixed', 'top:0', 'left:0', 'pointer-events:none',
        'z-index:2147483647', 'box-sizing:border-box',
        'border:2px solid #6366f1', 'background:rgba(99,102,241,0.08)',
        'transition:all 0.1s ease', 'border-radius:3px',
        'display:none',
    ].join(';');
    document.body.appendChild(overlay);

    const label = document.createElement('div');
    label.style.cssText = [
        'position:fixed', 'z-index:2147483648', 'background:#6366f1',
        'color:#fff', 'font:11px/1.4 monospace', 'padding:2px 6px',
        'border-radius:3px', 'pointer-events:none', 'display:none',
        'max-width:300px', 'white-space:nowrap', 'overflow:hidden',
        'text-overflow:ellipsis',
    ].join(';');
    document.body.appendChild(label);

    // ── Helpers ──────────────────────────────────────────────────────────────
    function getSelector(el) {
        if (!el || el === document.body) return 'body';
        const parts = [];
        let cur = el;
        while (cur && cur !== document.body) {
            let part = cur.tagName.toLowerCase();
            if (cur.id) {
                part += '#' + cur.id;
                parts.unshift(part);
                break;
            }
            if (cur.className) {
                const classes = Array.from(cur.classList).slice(0, 2).join('.');
                if (classes) part += '.' + classes;
            }
            const siblings = cur.parentElement
                ? Array.from(cur.parentElement.children).filter(c => c.tagName === cur.tagName)
                : [];
            if (siblings.length > 1) {
                const idx = siblings.indexOf(cur) + 1;
                part += ':nth-of-type(' + idx + ')';
            }
            parts.unshift(part);
            cur = cur.parentElement;
        }
        return parts.join(' > ');
    }

    function getReactComponent(el) {
        // Try to find React fiber (React DevTools approach)
        const key = Object.keys(el).find(k => k.startsWith('__reactFiber') || k.startsWith('__reactInternals'));
        if (!key) return null;
        let fiber = el[key];
        while (fiber) {
            const name = fiber.type && (typeof fiber.type === 'string' ? fiber.type : (fiber.type.displayName || fiber.type.name));
            if (name && name !== 'div' && name !== 'span' && !/^[a-z]/.test(name)) return name;
            fiber = fiber.return;
        }
        return null;
    }

    function getStyles(el) {
        const cs = window.getComputedStyle(el);
        return {
            fontSize: cs.fontSize,
            color: cs.color,
            backgroundColor: cs.backgroundColor,
            padding: cs.padding,
            margin: cs.margin,
            fontWeight: cs.fontWeight,
            display: cs.display,
            flexDirection: cs.flexDirection,
        };
    }

    function buildInfo(el) {
        const rect = el.getBoundingClientRect();
        return {
            selector: getSelector(el),
            outerHTML: el.outerHTML.substring(0, 500),
            tagName: el.tagName.toLowerCase(),
            reactComponent: getReactComponent(el),
            boundingRect: { top: rect.top, left: rect.left, width: rect.width, height: rect.height },
            styles: getStyles(el),
        };
    }

    function showOverlay(el) {
        const rect = el.getBoundingClientRect();
        overlay.style.cssText += ';display:block';
        overlay.style.top = rect.top + 'px';
        overlay.style.left = rect.left + 'px';
        overlay.style.width = rect.width + 'px';
        overlay.style.height = rect.height + 'px';
        overlay.style.display = 'block';

        const rc = getReactComponent(el);
        const name = rc || el.tagName.toLowerCase();
        label.textContent = '<' + name + '>';
        label.style.top = Math.max(0, rect.top - 20) + 'px';
        label.style.left = rect.left + 'px';
        label.style.display = 'block';
    }

    function hideOverlay() {
        overlay.style.display = 'none';
        label.style.display = 'none';
    }

    // ── Event listeners ──────────────────────────────────────────────────────
    document.addEventListener('mouseover', function (e) {
        if (e.target === overlay || e.target === label) return;
        showOverlay(e.target);
        window.parent.postMessage({ type: 'vibe:element-hovered', data: buildInfo(e.target) }, '*');
    }, true);

    document.addEventListener('mouseout', function (e) {
        if (!e.relatedTarget || e.relatedTarget === document.body) {
            hideOverlay();
        }
    }, true);

    document.addEventListener('click', function (e) {
        e.preventDefault();
        e.stopPropagation();
        const info = buildInfo(e.target);
        window.parent.postMessage({ type: 'vibe:element-selected', data: info }, '*');
    }, true);

    // Signal to parent that inspector is ready
    window.parent.postMessage({ type: 'vibe:inspector-ready' }, '*');

    // Listen for deactivation command
    window.addEventListener('message', function (e) {
        if (e.data && e.data.type === 'vibe:deactivate-inspector') {
            document.removeEventListener('mouseover', arguments.callee, true);
            document.removeEventListener('click', arguments.callee, true);
            hideOverlay();
            overlay.remove();
            label.remove();
            delete window.__vibeInspectorActive;
        }
    });
})();
