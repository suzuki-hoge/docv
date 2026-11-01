// docv build の HTML の JS。build が出力先の直下に docv.js として書き、すべてのページが defer で読む。
// 機能ごとに、対象の要素があるページでだけ動く。JS が動かなくても、すべての中身は読め、リンクから辿れる。

// Slot、ADR、Free ファイルのページ。h1 の上の元のファイルのパスを、ボタンでクリップボードにコピーする。
// ボタンは JS が動くときだけ出す。コピーしたら、少しの間ボタンを済みの印にする。
(function () {
  var button = document.querySelector('.source .copy');
  if (!button) return;
  var label = button.getAttribute('aria-label');
  button.hidden = false;

  function done() {
    button.classList.add('copied');
    button.title = 'コピーした';
    button.setAttribute('aria-label', 'コピーした');
    setTimeout(function () {
      button.classList.remove('copied');
      button.title = label;
      button.setAttribute('aria-label', label);
    }, 1500);
  }

  // クリップボードの API が使えないブラウザでは、見えない入力欄に入れて選び、コピーする。
  function fallback(text) {
    var area = document.createElement('textarea');
    area.value = text;
    area.setAttribute('readonly', '');
    area.style.position = 'fixed';
    area.style.opacity = '0';
    document.body.appendChild(area);
    area.select();
    var copied = false;
    try {
      copied = document.execCommand('copy');
    } catch (e) {}
    document.body.removeChild(area);
    if (copied) done();
  }

  button.addEventListener('click', function () {
    var text = button.getAttribute('data-path');
    if (navigator.clipboard && window.isSecureContext) {
      navigator.clipboard.writeText(text).then(done, function () {
        fallback(text);
      });
    } else {
      fallback(text);
    }
  });
})();

// 決定のページ。カーソルキーの ← と → を、前後の決定のカードへのリンクにする。
// 入力欄にいるときと、修飾キーを押しているときは、ブラウザの動きのままにする。
(function () {
  if (!document.querySelector('.adr-nav')) return;
  document.addEventListener('keydown', function (e) {
    if (e.altKey || e.ctrlKey || e.metaKey || e.shiftKey || /^(INPUT|TEXTAREA|SELECT)$/.test(e.target.tagName)) return;
    var side = { ArrowLeft: 'prev', ArrowRight: 'next' }[e.key];
    var a = side && document.querySelector('.adr-nav .' + side + ' a');
    if (a) location.href = a.href;
  });
})();

// プロジェクト詳細のタグのトグル。選んだタグのどれかを持つ Free ファイルの行と、その上のディレクトリの行だけを出す。
// すべて選んでいるときに押したタグは、そのタグだけを選ぶ。それ以外のときは押したタグを切り替え、1 つも残らなければすべて選び直す。
// 絞り込んでも列の幅が変わらないように、すべての行を出した状態のファイルとタグの列の幅で固定する。
(function () {
  var table = document.querySelector('table.docs');
  var buttons = [].slice.call(document.querySelectorAll('.tag-filter button'));
  if (!table || !buttons.length) return;
  var heads = table.tHead.rows[0].cells;
  var fileWidth = heads[0].offsetWidth;
  var tagWidth = heads[2].offsetWidth;
  heads[0].style.width = fileWidth + 'px';
  heads[2].style.width = tagWidth + 'px';
  table.style.tableLayout = 'fixed';
  var files = [].slice.call(table.querySelectorAll('tr[data-path]'));
  var dirs = [].slice.call(table.querySelectorAll('tr[data-dir]'));

  function pressed(b) {
    return b.getAttribute('aria-pressed') === 'true';
  }

  function set(b, on) {
    b.setAttribute('aria-pressed', on ? 'true' : 'false');
  }

  function apply() {
    var on = buttons.filter(pressed).map(function (b) {
      return b.getAttribute('data-tag');
    });
    var all = on.length === buttons.length;
    files.forEach(function (r) {
      var tags = JSON.parse(r.getAttribute('data-tags'));
      r.hidden = !all && !tags.some(function (t) {
        return on.indexOf(t) >= 0;
      });
    });
    dirs.forEach(function (d) {
      var prefix = d.getAttribute('data-dir') + '/';
      d.hidden = !files.some(function (r) {
        return !r.hidden && r.getAttribute('data-path').indexOf(prefix) === 0;
      });
    });
  }

  buttons.forEach(function (b) {
    b.addEventListener('click', function () {
      if (buttons.every(pressed)) {
        buttons.forEach(function (x) {
          set(x, x === b);
        });
      } else {
        set(b, !pressed(b));
        if (!buttons.some(pressed)) {
          buttons.forEach(function (x) {
            set(x, true);
          });
        }
      }
      apply();
    });
  });
})();

// 検索のページ。入力した文字列をそのまま含む行を探し、ファイルごとに入力欄の下へ並べる。
// 索引 DOCV_SEARCH は、build が検索のページの中に書く。ファイルの並びは索引の順（Slot ファイル、ADR、Free ファイル）のまま。
(function () {
  var form = document.querySelector('form.search');
  if (!form || typeof DOCV_SEARCH === 'undefined') return;
  // ヒットした行の前後に出す行の数と、1 つのファイルに出すプレビューの数。
  var AROUND = 2;
  var MAX_PREVIEWS = 3;
  var input = form.querySelector('input');
  var count = document.getElementById('search-count');
  var results = document.getElementById('search-results');
  form.hidden = false;

  function escape(s) {
    return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
  }

  // 行の中の、入力した文字列に印を付ける。
  function mark(line, q) {
    return line.split(q).map(escape).join('<mark>' + escape(q) + '</mark>');
  }

  // ヒットした行の前後の範囲。重なるか隣り合う範囲は、1 つにつなげる。
  function ranges(lines, hits) {
    var out = [];
    hits.forEach(function (i) {
      var from = Math.max(0, i - AROUND);
      var to = Math.min(lines.length - 1, i + AROUND);
      var last = out[out.length - 1];
      if (last && from <= last.to + 1) {
        last.to = to;
      } else {
        out.push({ from: from, to: to });
      }
    });
    return out;
  }

  function file(f, q, hits) {
    var html = '<section class="hit"><p class="hit-file"><a href="' + escape(f.href) + '">' + escape(f.path) + '</a>';
    f.tags.forEach(function (t) {
      html += '<span class="tag c' + t[1] + '">' + escape(t[0]) + '</span>';
    });
    html += '</p>';
    var rs = ranges(f.lines, hits);
    rs.slice(0, MAX_PREVIEWS).forEach(function (r) {
      html += '<pre class="preview">';
      for (var i = r.from; i <= r.to; i++) {
        var line = f.lines[i];
        html += '<span class="ln">' + (i + 1) + '</span>' + (line.indexOf(q) >= 0 ? mark(line, q) : escape(line)) + '\n';
      }
      html += '</pre>';
    });
    if (rs.length > MAX_PREVIEWS) {
      html += '<p class="note">ほか ' + (rs.length - MAX_PREVIEWS) + ' か所</p>';
    }
    return html + '</section>';
  }

  function search() {
    var q = input.value;
    if (!q) {
      count.textContent = '';
      results.innerHTML = '';
      return;
    }
    var html = '';
    var files = 0;
    var lines = 0;
    DOCV_SEARCH.forEach(function (f) {
      var hits = [];
      f.lines.forEach(function (line, i) {
        if (line.indexOf(q) >= 0) hits.push(i);
      });
      if (!hits.length) return;
      files += 1;
      lines += hits.length;
      html += file(f, q, hits);
    });
    count.textContent = files ? files + ' ファイル、' + lines + ' 行' : '見つからない';
    results.innerHTML = html;
  }

  input.addEventListener('input', search);
  form.addEventListener('submit', function (e) {
    e.preventDefault();
  });
  // 戻るで開き直したときは、ブラウザが残した入力で探し直す。
  window.addEventListener('pageshow', search);
  search();
})();
