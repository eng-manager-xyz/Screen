// Storybook index cockpit (DEV-03 / DEV-08).
// Vanilla JS, no framework. Loaded inline from export_stories.rs.

(function () {
  var frame = document.getElementById('storybook-index-frame');
  var current = document.getElementById('storybook-index-current');
  var openLink = document.getElementById('storybook-index-open');
  var filter = document.getElementById('storybook-index-filter');
  var list = document.querySelector('.storybook-index-list');
  var FILTER_KEY = 'storybook-index-filter';

  function pick(id) {
    if (!id) {
      id = (document.body.getAttribute('data-first-story') || '').trim();
    }
    if (!id) return;
    var safeId = id.replace(/[^a-zA-Z0-9_-]/g, '');
    if (safeId !== id) return;
    var href = './' + safeId + '.html';
    if (frame.getAttribute('src') !== href) {
      frame.setAttribute('src', href);
    }
    if (current) current.textContent = safeId;
    if (openLink) openLink.setAttribute('href', href);
    document.querySelectorAll('.storybook-index-row').forEach(function (row) {
      if (row.getAttribute('data-id') === safeId) {
        row.classList.add('is-active');
      } else {
        row.classList.remove('is-active');
      }
    });
  }

  function readHash() {
    return decodeURIComponent((location.hash || '').replace(/^#/, ''));
  }

  window.addEventListener('hashchange', function () {
    pick(readHash());
  });

  pick(readHash());

  if (list) {
    list.addEventListener('click', function (e) {
      var target = e.target;
      while (target && target !== list && target.tagName !== 'A') {
        target = target.parentNode;
      }
      if (!target || target === list) return;
      var id = target.getAttribute('data-id');
      if (!id) return;
      // Allow native anchor to update the hash; pick() runs from hashchange.
    });
  }

  function applyFilter(query) {
    var q = (query || '').trim().toLowerCase();
    var anyVisible = false;
    document.querySelectorAll('.storybook-index-row').forEach(function (row) {
      var match =
        !q ||
        (row.getAttribute('data-id') || '').indexOf(q) !== -1 ||
        (row.getAttribute('data-title') || '').indexOf(q) !== -1 ||
        (row.getAttribute('data-category') || '').indexOf(q) !== -1;
      if (match) {
        row.classList.remove('is-hidden');
        anyVisible = true;
      } else {
        row.classList.add('is-hidden');
      }
    });
    document.querySelectorAll('.storybook-index-category').forEach(function (cat) {
      var rows = cat.querySelectorAll('.storybook-index-row');
      var visible = 0;
      rows.forEach(function (r) {
        if (!r.classList.contains('is-hidden')) visible += 1;
      });
      if (visible === 0) {
        cat.classList.add('is-hidden');
      } else {
        cat.classList.remove('is-hidden');
      }
    });
    try {
      window.sessionStorage.setItem(FILTER_KEY, q);
    } catch (_e) {
      /* sessionStorage may be unavailable (private mode) */
    }
    return anyVisible;
  }

  if (filter) {
    try {
      var stored = window.sessionStorage.getItem(FILTER_KEY) || '';
      if (stored) {
        filter.value = stored;
        applyFilter(stored);
      }
    } catch (_e) {
      /* ignore */
    }
    filter.addEventListener('input', function () {
      applyFilter(filter.value);
    });
    document.addEventListener('keydown', function (e) {
      var inField =
        e.target &&
        (e.target.tagName === 'INPUT' ||
          e.target.tagName === 'TEXTAREA' ||
          e.target.isContentEditable);
      if (e.key === '/' && !inField) {
        e.preventDefault();
        filter.focus();
        filter.select();
      } else if (e.key === 'Escape' && document.activeElement === filter) {
        filter.value = '';
        applyFilter('');
        filter.blur();
      }
    });
  }
})();
