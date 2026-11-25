// Pick light or dark before the first paint (app.js keeps it up to date afterwards). A file of its own, not a script
// written in index.html: the page's Content-Security-Policy runs no inline script at all.
// Inside the app the window already tells the page which scheme to use (Windows' own, unless Settings says
// otherwise), so only the media query is asked. The stored copy is for the plain-browser stand-in.
try {
  var inApp = !!(window.chrome && window.chrome.webview);
  var t = new URLSearchParams(location.search).get('theme') || (inApp ? 'system' : (localStorage.getItem('babel-theme') || localStorage.getItem('nm-theme'))) || 'system';
  document.documentElement.dataset.theme = (t === 'dark' || (t !== 'light' && matchMedia('(prefers-color-scheme: dark)').matches)) ? 'dark' : 'light';
} catch (e) { document.documentElement.dataset.theme = 'light'; }
