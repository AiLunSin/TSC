/*
 * README 动图的演示脚本：模拟一次"选中 → 查意思 → 生成卡片"。
 *
 * 和 mock.js 一样，必须作为独立文件加载（页面 CSP 是 script-src 'self'）。
 * 真实用户是鼠标划选，这里用 Range + mouseup 事件走同一条前端逻辑，不直接改 React 状态。
 */
(function () {
  function selectLine() {
    var body = document.querySelector(".transcript-body");
    if (!body) {
      return;
    }
    var lines = body.querySelectorAll(".transcript-line");
    if (lines.length < 4) {
      return;
    }
    var range = document.createRange();
    range.selectNodeContents(lines[3]);
    var selection = window.getSelection();
    selection.removeAllRanges();
    selection.addRange(range);
    // 前端靠这个事件把选区收进状态，和真人放开鼠标时一样。
    body.dispatchEvent(new MouseEvent("mouseup", { bubbles: true, cancelable: true }));
  }

  function clickByText(text) {
    var buttons = Array.prototype.slice.call(document.querySelectorAll("button"));
    for (var i = 0; i < buttons.length; i += 1) {
      if (buttons[i].textContent.trim() === text && !buttons[i].disabled) {
        buttons[i].click();
        return true;
      }
    }
    return false;
  }

  setTimeout(selectLine, 900);
  setTimeout(function () {
    clickByText("查意思");
  }, 1900);
  setTimeout(function () {
    clickByText("生成卡片");
  }, 3000);
})();
