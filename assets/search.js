// docv build の検索のページの JS。入力した文字列をそのまま含む行を探し、ファイルごとに入力欄の下へ並べる。
// 索引 DOCV_SEARCH は、build がこの JS の前に書く。ファイルの並びは索引の順（Slot ファイル、ADR、Free ファイル）のまま。
(function () {
  // ヒットした行の前後に出す行の数と、1 つのファイルに出すプレビューの数。
  var AROUND = 2;
  var MAX_PREVIEWS = 3;
  var form = document.querySelector('form.search');
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
