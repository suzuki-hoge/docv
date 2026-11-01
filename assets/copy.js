// docv build の Slot、ADR、Free ファイルのページの JS。h1 の上の元のファイルのパスを、ボタンでクリップボードにコピーする。
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
