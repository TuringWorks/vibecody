// VibeCody Docs — TOC generation, search, keyboard shortcuts

(function () {
  'use strict';

  // ── Table of Contents Generation ───────────────────────────────────
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
      // Ensure heading has an ID for linking
      if (!heading.id) {
        heading.id = heading.textContent
          .toLowerCase()
          .replace(/[^a-z0-9]+/g, '-')
          .replace(/^-|-$/g, '');
      }

      var link = document.createElement('a');
      link.href = '#' + heading.id;
      link.textContent = heading.textContent;
      if (heading.tagName === 'H3') {
        link.classList.add('toc-h3');
      }
      tocNav.appendChild(link);
    });

    // Highlight active TOC item on scroll
    var observer = new IntersectionObserver(function (entries) {
      entries.forEach(function (entry) {
        if (entry.isIntersecting) {
          var links = tocNav.querySelectorAll('a');
          links.forEach(function (l) { l.classList.remove('active'); });
          var activeLink = tocNav.querySelector('a[href="#' + entry.target.id + '"]');
          if (activeLink) activeLink.classList.add('active');
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

    // Keyboard shortcut: / to focus search
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

    // Build a simple search index from sidebar links
    var searchIndex = [];
    document.querySelectorAll('.nav-link, .nav-link.child').forEach(function (link) {
      searchIndex.push({
        title: link.textContent.trim(),
        url: link.getAttribute('href') || '#',
      });
    });

    input.addEventListener('focus', function () {
      if (input.value.trim()) overlay.style.display = 'flex';
    });

    input.addEventListener('input', function () {
      var query = input.value.toLowerCase().trim();
      if (!query) {
        overlay.style.display = 'none';
        return;
      }
      overlay.style.display = 'flex';

      var matches = searchIndex.filter(function (item) {
        return item.title.toLowerCase().includes(query);
      });

      results.innerHTML = matches.length === 0
        ? '<div style="padding:20px;text-align:center;color:#8888a0">No results for "' + query + '"</div>'
        : matches.map(function (m) {
            return '<a href="' + m.url + '" class="search-result-item" style="display:block;text-decoration:none">'
              + '<div class="search-result-title">' + m.title + '</div>'
              + '<div class="search-result-url">' + m.url + '</div></a>';
          }).join('');
    });

    overlay.addEventListener('click', function (e) {
      if (e.target === overlay) overlay.style.display = 'none';
    });
  }

  // ── Smooth scroll for anchor links ─────────────────────────────────
  function initSmoothScroll() {
    document.querySelectorAll('a[href^="#"]').forEach(function (a) {
      a.addEventListener('click', function (e) {
        var target = document.getElementById(this.getAttribute('href').slice(1));
        if (target) {
          e.preventDefault();
          target.scrollIntoView({ behavior: 'smooth', block: 'start' });
          history.pushState(null, '', this.getAttribute('href'));
        }
      });
    });
  }

  // ── Init ───────────────────────────────────────────────────────────
  document.addEventListener('DOMContentLoaded', function () {
    generateTOC();
    initSearch();
    initSmoothScroll();
  });
})();
