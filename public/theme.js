// Apply before first paint. An explicit choice wins over the system preference.
(() => {
  let preference; try { preference = localStorage.getItem('folio-theme'); } catch {}
  const dark = preference ? preference === 'dark' : matchMedia('(prefers-color-scheme: dark)').matches;
  document.documentElement.dataset.theme = dark ? 'dark' : 'light';
})();
