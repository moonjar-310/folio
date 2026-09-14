// Run through the Penpot MCP execute_code tool after the native component import.
const f = storage.folio;
for (const theme of ['light', 'dark']) {
  const x = theme === 'light' ? 1800 : 2200;
  const side = f.board('Navigation / Sidebar / ' + theme, x, 100, 240, 1024, penpot.root, theme, 'well');
  f.text(side, 'Brand', 25, 20, '♧  Folio', 26, 'ink', theme, 'Newsreader', 180);
  f.instance(side, 'Input/Search', 12, 88, theme);
  for (const [i, name] of ['Home', 'Planner', 'Todo', 'Notes'].entries()) f.instance(side, 'Navigation/Item', 12, 146 + i * 43, theme, { Home: name });
  f.text(side, 'Folder heading', 25, 352, 'FOLDERS', 10, 'muted', theme, 'JetBrains Mono', 170);
  for (const [i, name] of ['Daily', 'Projects', 'Personal', 'Archive'].entries()) f.text(side, 'Folder / ' + name, 26, 390 + i * 38, '▱   ' + name, 13, 'muted', theme, 'Geist', 190);
  f.instance(side, 'Action/Primary', 16, 856, theme, { 'Create note': '+ New note' });
  f.text(side, 'Theme toggle', 26, 916, theme === 'dark' ? '◐   Light mode' : '◐   Dark mode', 13, 'muted', theme, 'Geist', 190);
  f.text(side, 'Settings', 26, 960, '⚙   Settings', 13, 'muted', theme, 'Geist', 190);
  f.register(side, 'Navigation/Sidebar', theme);
  const top = f.board('Navigation / Topbar / ' + theme, x, 1200, 1040, 68, penpot.root, theme);
  f.text(top, 'Breadcrumb', 28, 25, 'Personal workspace  /  Home', 12, 'muted', theme, 'Geist', 520);
  f.instance(top, 'Status/Saved', 850, 23, theme);
  f.rect(top, 'Rule', 0, 67, 1040, 1, 'line', theme);
  f.register(top, 'Navigation/Topbar', theme);
}
f.screen = (name, x, y, theme, height = 1240) => {
  const board = f.board(name + ' / ' + theme, x, y, 1280, height, penpot.root, theme);
  const sidebar = f.instance(board, 'Navigation/Sidebar', 0, 0, theme);
  sidebar.resize(240, height);
  const top = f.instance(board, 'Navigation/Topbar', 240, 0, theme, { 'Personal workspace  /  Home': 'Personal workspace  /  ' + name });
  const nav = penpotUtils.findShapes(s => s.isComponentCopyInstance() && s.component()?.name === 'Item', sidebar);
  const index = ['Home', 'Planner', 'Todo', 'Notes'].indexOf(name);
  if (nav[index]) nav[index].fills = [{fillColor: f.colors[theme].selected, fillOpacity: 1}];
  return board;
};
f.row = (board, x, y, text, theme, complete = false, time = '09:00') => f.instance(board, complete ? 'Task/Completed' : 'Task/Open', x, y, theme, { 'Review the authentication architecture': text, 'Morning pages & a slow coffee': text, '09:00': time, '07:30': time });
return { components: penpot.library.local.components.length };
