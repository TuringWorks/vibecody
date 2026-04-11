// VibeCody Docs — TOC, search, theme toggle, keyboard shortcuts

(function () {
  'use strict';

  // ── Theme Toggle ───────────────────────────────────────────────────
  function initTheme() {
    var toggle = document.getElementById('theme-toggle');
    if (!toggle) return;

    toggle.addEventListener('click', function () {
      var html = document.documentElement;
      var current = html.getAttribute('data-theme') || 'dark';
      var next = current === 'dark' ? 'light' : 'dark';
      html.setAttribute('data-theme', next);
      localStorage.setItem('vibecody-theme', next);
    });
  }

  // ── Table of Contents ──────────────────────────────────────────────
  function generateTOC() {
    var tocNav = document.getElementById('toc-nav');
    if (!tocNav) return;

    var content = document.querySelector('.page-content');
    if (!content) return;

    var headings = content.querySelectorAll('h2, h3');
    if (headings.length === 0) {
      var tocSidebar = document.getElementById('toc-sidebar');
      if (tocSidebar) tocSidebar.style.display = 'none';
      return;
    }

    headings.forEach(function (heading) {
      if (!heading.id) {
        heading.id = heading.textContent
          .toLowerCase()
          .replace(/[^a-z0-9]+/g, '-')
          .replace(/^-|-$/g, '');
      }

      var link = document.createElement('a');
      link.href = '#' + heading.id;
      link.textContent = heading.textContent;
      if (heading.tagName === 'H3') link.classList.add('toc-h3');
      tocNav.appendChild(link);
    });

    // Active heading tracking
    var observer = new IntersectionObserver(function (entries) {
      entries.forEach(function (entry) {
        if (entry.isIntersecting) {
          var links = tocNav.querySelectorAll('a');
          links.forEach(function (l) { l.classList.remove('active'); });
          var active = tocNav.querySelector('a[href="#' + entry.target.id + '"]');
          if (active) active.classList.add('active');
        }
      });
    }, { rootMargin: '-80px 0px -80% 0px' });

    headings.forEach(function (h) { observer.observe(h); });
  }

  // ── Search ─────────────────────────────────────────────────────────
  function initSearch() {
    var input = document.getElementById('search-input');
    var overlay = document.getElementById('search-overlay');
    var results = document.getElementById('search-results');
    if (!input || !overlay || !results) return;

    // Build search index from sidebar
    var searchIndex = [];
    document.querySelectorAll('.sidebar-nav .nav-link, .sidebar-nav .nav-group-toggle').forEach(function (el) {
      var href = el.getAttribute('href') || '#';
      var text = el.textContent.trim();
      if (text && href !== '#') {
        searchIndex.push({ title: text, url: href });
      }
    });

    // "/" to focus
    document.addEventListener('keydown', function (e) {
      if (e.key === '/' && !e.ctrlKey && !e.metaKey) {
        var tag = document.activeElement.tagName;
        if (tag !== 'INPUT' && tag !== 'TEXTAREA') {
          e.preventDefault();
          input.focus();
        }
      }
      if (e.key === 'Escape') {
        overlay.style.display = 'none';
        input.blur();
      }
    });

    input.addEventListener('focus', function () {
      if (input.value.trim()) overlay.style.display = 'flex';
    });

    input.addEventListener('input', function () {
      var query = input.value.toLowerCase().trim();
      if (!query) { overlay.style.display = 'none'; return; }
      overlay.style.display = 'flex';

      var matches = searchIndex.filter(function (item) {
        return item.title.toLowerCase().includes(query);
      });

      results.textContent = '';
      if (matches.length === 0) {
        var empty = document.createElement('div');
        empty.style.cssText = 'padding:20px;text-align:center;color:var(--text-muted)';
        empty.textContent = 'No results for \u201c' + query + '\u201d';
        results.appendChild(empty);
      } else {
        matches.forEach(function (m) {
          var a = document.createElement('a');
          a.href = m.url;
          a.className = 'search-result-item';
          var title = document.createElement('div');
          title.className = 'search-result-title';
          title.textContent = m.title;
          var url = document.createElement('div');
          url.className = 'search-result-url';
          url.textContent = m.url;
          a.appendChild(title);
          a.appendChild(url);
          results.appendChild(a);
        });
      }
    });

    overlay.addEventListener('click', function (e) {
      if (e.target === overlay) overlay.style.display = 'none';
    });
  }

  // ── Smooth Scroll ──────────────────────────────────────────────────
  function initSmoothScroll() {
    document.addEventListener('click', function (e) {
      var link = e.target.closest('a[href^="#"]');
      if (!link) return;
      var target = document.getElementById(link.getAttribute('href').slice(1));
      if (target) {
        e.preventDefault();
        target.scrollIntoView({ behavior: 'smooth', block: 'start' });
        history.pushState(null, '', link.getAttribute('href'));
      }
    });
  }

  // ── Image Lightbox ─────────────────────────────────────────────────
  function initLightbox() {
    // Build overlay DOM once
    var overlay = document.createElement('div');
    overlay.id = 'lightbox-overlay';
    overlay.setAttribute('role', 'dialog');
    overlay.setAttribute('aria-modal', 'true');
    overlay.setAttribute('aria-label', 'Image preview');

    var img = document.createElement('img');
    img.id = 'lightbox-img';
    img.alt = '';

    var caption = document.createElement('div');
    caption.id = 'lightbox-caption';

    var close = document.createElement('button');
    close.id = 'lightbox-close';
    close.setAttribute('aria-label', 'Close');
    close.textContent = '\u00d7'; // ×

    overlay.appendChild(close);
    overlay.appendChild(img);
    overlay.appendChild(caption);
    document.body.appendChild(overlay);

    function openLightbox(src, alt) {
      img.src = src;
      img.alt = alt || '';
      caption.textContent = alt || '';
      overlay.classList.add('is-open');
      document.body.style.overflow = 'hidden';
      close.focus();
    }

    function closeLightbox() {
      overlay.classList.remove('is-open');
      document.body.style.overflow = '';
      img.src = '';
    }

    // Wire up all content images (skip tiny icons)
    document.querySelectorAll('.page-content img, .post-content img').forEach(function (el) {
      el.style.cursor = 'zoom-in';
      el.addEventListener('click', function () {
        openLightbox(el.src, el.alt);
      });
    });

    close.addEventListener('click', closeLightbox);
    overlay.addEventListener('click', function (e) {
      if (e.target === overlay) closeLightbox();
    });
    document.addEventListener('keydown', function (e) {
      if (e.key === 'Escape' && overlay.classList.contains('is-open')) closeLightbox();
    });
  }

  // ── Init ───────────────────────────────────────────────────────────
  document.addEventListener('DOMContentLoaded', function () {
    initTheme();
    generateTOC();
    initSearch();
    initSmoothScroll();
    initLightbox();
  });
})();
