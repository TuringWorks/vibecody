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

      results.innerHTML = matches.length === 0
        ? '<div style="padding:20px;text-align:center;color:var(--text-muted)">No results for "' + query + '"</div>'
        : matches.map(function (m) {
            return '<a href="' + m.url + '" class="search-result-item">'
              + '<div class="search-result-title">' + m.title + '</div>'
              + '<div class="search-result-url">' + m.url + '</div></a>';
          }).join('');
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

  // ── Init ───────────────────────────────────────────────────────────
  document.addEventListener('DOMContentLoaded', function () {
    initTheme();
    generateTOC();
    initSearch();
    initSmoothScroll();
  });
})();
