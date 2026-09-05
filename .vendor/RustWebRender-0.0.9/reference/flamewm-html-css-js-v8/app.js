(() => {
  'use strict';

  const PANEL_HEIGHT = 44;
  const EDGE_DWELL_MS = 520;
  const SNAP_EDGE = 18;
  const GRID_X = 92;
  const GRID_Y = 91;
  const ICON_W = 82;
  const ICON_H = 82;
  const DEFAULT_ACCENT = '#ef4048';
  const DEFAULT_WALLPAPER = 'assets/images/flamewm-default-wallpaper.webp';
  const DEFAULT_HOTKEYS = Object.freeze({
    start: 'Meta',
    desktopLeft: 'Ctrl+Meta+ArrowLeft',
    desktopRight: 'Ctrl+Meta+ArrowRight',
    desktopUp: 'Ctrl+Meta+ArrowUp',
    desktopDown: 'Ctrl+Meta+ArrowDown',
  });

  const storageGet = (key, fallback = null) => { try { const v = window.localStorage.getItem(key); return v === null ? fallback : v; } catch (_) { return fallback; } };
  const storageSet = (key, value) => { try { window.localStorage.setItem(key, String(value)); } catch (_) {} };
  const storageGetJson = (key, fallback) => { try { return JSON.parse(storageGet(key, JSON.stringify(fallback))); } catch (_) { return fallback; } };

  const state = {
    desktops: Number(storageGet('flamewm-v8-desktops', '4')),
    currentDesktop: 1,
    windows: [],
    focusedWindowId: null,
    nextWindowId: 1,
    zCounter: 100,
    volume: Number(storageGet('flamewm-v8-volume', '72')),
    muted: false,
    mediaPlaying: true,
    wifiConnected: 'ArkNet 5G',
    openPopover: null,
    selectedDesktopIcons: new Set(),
    drag: null,
    resize: null,
    edgeTimer: null,
    edgeDirection: null,
    snapCandidate: null,
    accent: storageGet('flamewm-v8-accent', DEFAULT_ACCENT),
    customAccentCandidate: storageGet('flamewm-v8-custom-accent', '#d92857'),
    wallpaperData: storageGet('flamewm-v8-wallpaper', DEFAULT_WALLPAPER),
    showWatermark: storageGet('flamewm-v8-show-watermark', '1') !== '0',
    settingsPage: 'appearance',
    hotkeys: Object.assign({}, DEFAULT_HOTKEYS, storageGetJson('flamewm-v8-hotkeys', DEFAULT_HOTKEYS)),
    recordingHotkey: null,
    metaChordUsed: false,
    taskbarPosition: storageGet('flamewm-v8-taskbar-position', 'bottom'),
    taskbarDockCandidate: null,
    trashItems: storageGetJson('flamewm-v8-trash-items', []),
    fontFamily: storageGet('flamewm-v8-font-family', 'IBM Plex Sans'),
    fontBold: storageGet('flamewm-v8-font-bold', '0') === '1',
    fontSizeOffset: Number(storageGet('flamewm-v8-font-size-offset', '0')),
    selectionOpacity: Number(storageGet('flamewm-v8-selection-opacity', '20')),
    snapPreviewOpacity: Number(storageGet('flamewm-v8-snap-preview-opacity', '20')),
    selectedDisplayId: storageGet('flamewm-v8-selected-display', 'eDP-1'),
    displaySettings: storageGetJson('flamewm-v8-display-settings', {
      'eDP-1': { label:'Built-in Display', resolution:'1920x1080', scale:100, primary:true },
      'HDMI-1': { label:'External Display', resolution:'2560x1440', scale:100, primary:false }
    }),
    stickyNotesEnabled: storageGet('flamewm-v8-sticky-notes-enabled', '1') !== '0',
    stickyNotes: storageGetJson('flamewm-v8-sticky-notes', []),
    taskbarColor: storageGet('flamewm-v8-taskbar-color', '#191b1d'),
    taskbarOpacity: Number(storageGet('flamewm-v8-taskbar-opacity', '99')),
    taskbarHeight: Number(storageGet('flamewm-v8-taskbar-height', '44')),
    startButtonText: storageGet('flamewm-v8-start-button-text', ''),
    customStartIcon: storageGet('flamewm-v8-start-icon', ''),
    iconTheme: storageGet('flamewm-v8-icon-theme', 'FlameWM Breeze (Built-in)'),
    taskEntryOrder: storageGetJson('flamewm-v8-task-entry-order', []),
    taskDrag: null,
    pinnedAnchorWindow: {},
  };
  state.desktops = Math.max(1, Math.min(18, state.desktops));
  state.selectionOpacity = Math.max(0, Math.min(60, Number.isFinite(state.selectionOpacity) ? state.selectionOpacity : 20));
  state.snapPreviewOpacity = Math.max(0, Math.min(60, Number.isFinite(state.snapPreviewOpacity) ? state.snapPreviewOpacity : 20));
  state.taskbarOpacity = Math.max(0, Math.min(100, Number.isFinite(state.taskbarOpacity) ? state.taskbarOpacity : 99));
  state.taskbarHeight = Math.max(34, Math.min(72, Number.isFinite(state.taskbarHeight) ? state.taskbarHeight : 44));
  if (!['bottom','top','left','right'].includes(state.taskbarPosition)) state.taskbarPosition = 'bottom';
  if (!state.displaySettings[state.selectedDisplayId]) state.selectedDisplayId = Object.keys(state.displaySettings)[0] || 'eDP-1';

  const el = {
    desktop: document.getElementById('desktop'),
    wallpaper: document.getElementById('wallpaper'),
    watermark: document.getElementById('flamewm-watermark'),
    grid: document.getElementById('desktop-grid'),
    windows: document.getElementById('window-layer'),
    snap: document.getElementById('snap-preview'),
    taskbar: document.getElementById('taskbar'),
    start: document.getElementById('start-menu'),
    startButton: document.getElementById('start-button'),
    tasks: document.getElementById('task-entries'),
    pager: document.getElementById('workspace-pager'),
    mediaButton: document.getElementById('media-button'),
    audioButton: document.getElementById('audio-button'),
    wifiButton: document.getElementById('wifi-button'),
    clockButton: document.getElementById('clock-button'),
    mediaPopup: document.getElementById('media-popup'),
    audioPopup: document.getElementById('audio-popup'),
    wifiPopup: document.getElementById('wifi-popup'),
    clockPopup: document.getElementById('clock-popup'),
    desktopMenu: document.getElementById('desktop-menu'),
    workspaceMenu: document.getElementById('workspace-menu'),
    taskMenu: document.getElementById('task-menu'),
    entryMenu: document.getElementById('entry-menu'),
    taskbarMenu: document.getElementById('taskbar-menu'),
    stickyMenu: document.getElementById('sticky-menu'),
    stickyLayer: document.getElementById('sticky-note-layer'),
    dockPreview: document.getElementById('taskbar-dock-preview'),
    selectionRect: document.getElementById('desktop-selection-rectangle'),
  };

  const iconDefs = Object.freeze({
    start: ['flamewm-icon.svg', 'flame'],
    browser: ['internet-web-browser.svg', 'img'],
    files: ['system-file-manager.svg', 'img'],
    folder: ['folder.svg', 'img'],
    terminal: ['utilities-terminal.svg', 'img'],
    settings: ['preferences-system.svg', 'img'],
    music: ['elisa.svg', 'img'],
    code: ['applications-development.svg', 'img'],
    game: ['applications-games.svg', 'img'],
    graphics: ['applications-graphics.svg', 'img'],
    internet: ['internet-visible.svg', 'mono'],
    multimedia: ['applications-multimedia.svg', 'img'],
    system: ['applications-system.svg', 'img'],
    utility: ['applications-utilities.svg', 'img'],
    wifi: ['network-wireless.svg', 'mono'],
    volume: ['audio-volume-high.svg', 'mono'],
    muted: ['audio-volume-muted.svg', 'mono'],
    media: ['multimedia-player.svg', 'mono'],
    play: ['media-playback-start.svg', 'mono'],
    pause: ['media-playback-pause.svg', 'mono'],
    prev: ['media-skip-backward.svg', 'mono'],
    next: ['media-skip-forward.svg', 'mono'],
    search: ['search-visible.svg', 'mono'],
    chevron: ['go-next.svg', 'mask'],
    plus: ['list-add-visible.svg', 'mono'],
    minus: ['window-minimize.svg', 'mono'],
    close: ['window-close.svg', 'mono'],
    maximize: ['window-maximize.svg', 'mono'],
    restore: ['window-restore.svg', 'mono'],
    power: ['system-shutdown-visible.svg', 'mono'],
    reboot: ['system-reboot-visible.svg', 'mono'],
    logout: ['system-log-out-visible.svg', 'mono'],
    lock: ['system-lock-screen-visible.svg', 'mono'],
    info: ['help-about.svg', 'mask'],
    trash: ['user-trash.svg', 'img'],
    home: ['user-home.svg', 'img'],
    download: ['folder-downloads.svg', 'img'],
    documents: ['folder-documents.svg', 'img'],
    pictures: ['folder-pictures.svg', 'img'],
    videos: ['folder-videos.svg', 'img'],
    run: ['system-run.svg', 'mask'],
    monitor: ['user-desktop-visible.svg', 'mono'],
    desktop: ['preferences-desktop.svg', 'mono'],
    color: ['preferences-desktop-color.svg', 'mask'],
    wallpaper: ['preferences-desktop-wallpaper.svg', 'mono'],
    keyboard: ['input-keyboard.svg', 'mask'],
    calendar: ['view-calendar.svg', 'mask'],
    arrowLeft: ['go-previous.svg', 'mask'],
    arrowRight: ['go-next.svg', 'mask'],
    menu: ['application-menu.svg', 'mask'],
    configure: ['configure.svg', 'mask'],
    folderNew: ['folder-new-visible.svg', 'mono'],
    globe: ['network-workgroup-visible.svg', 'mono'],
    systemDisk: ['drive-harddisk-visible.svg', 'mono'],
    display: ['preferences-desktop-display.svg', 'mono'],
    font: ['preferences-desktop-font.svg', 'mono'],
    settingsAppearance: ['settings-appearance.svg', 'img'],
    settingsDesktop: ['settings-desktop.svg', 'img'],
    settingsDisplays: ['settings-displays.svg', 'mono'],
    settingsFonts: ['settings-fonts.svg', 'mono'],
    settingsHotkeys: ['settings-hotkeys.svg', 'mono'],
    settingsAbout: ['settings-about.svg', 'img'],
    undo: ['edit-undo.svg', 'mask'],
    github: ['github.svg', 'asset'],
    paypal: ['paypal.svg', 'asset'],
    website: ['internet-web-browser.svg', 'img'],
    ark: ['ark.svg', 'img'],
    kate: ['kate.svg', 'img'],
    kcalc: ['kcalc.svg', 'img'],
    spectacle: ['spectacle.svg', 'img'],
    okular: ['okular.svg', 'img'],
    gwenview: ['gwenview.svg', 'img'],
    kdenlive: ['kdenlive.svg', 'img'],
    kwrite: ['kwrite.svg', 'img'],
    sticky: ['sticky-note.svg', 'asset'],
  });

  const svg = (name, cls = '') => {
    const def = iconDefs[name] || iconDefs.info;
    const classes = cls ? ` ${cls}` : '';
    if (def[1] === 'flame') return `<img class="breeze-img flamewm-start-icon${classes}" src="assets/flamewm-icon.svg" alt="" draggable="false">`;
    if (def[1] === 'asset') return `<img class="breeze-img brand-icon${classes}" src="assets/${def[0]}" alt="" draggable="false">`;
    const path = `assets/breeze/${def[0]}`;
    if (def[1] === 'img') return `<img class="breeze-img${classes}" src="${path}" alt="" draggable="false">`;
    if (def[1] === 'mono') return `<img class="breeze-img mono${classes}" src="${path}" alt="" draggable="false">`;
    return `<span class="breeze-icon${classes}" style="--icon:url('${path}')" aria-hidden="true"></span>`;
  };

  const applications = {
    files: { id:'files', name:'Dolphin', description:'File Manager', icon:'files', category:'System', width:760, height:490, content: renderFiles },
    browser: { id:'browser', name:'Firefox', description:'Web Browser', icon:'browser', category:'Internet', width:820, height:520, content: renderBrowser },
    terminal: { id:'terminal', name:'Konsole', description:'Terminal', icon:'terminal', category:'System', width:650, height:390, content: renderTerminal },
    settings: { id:'settings', name:'System Settings', description:'Configure your desktop', icon:'settings', category:'System', width:720, height:480, content: renderSettings },
    music: { id:'music', name:'Elisa', description:'Music Player', icon:'music', category:'Multimedia', width:590, height:410, content: renderMusic },
    code: { id:'code', name:'Visual Studio Code', description:'Code Editor', icon:'code', category:'Development', width:790, height:500, content: renderCode },
    game: { id:'game', name:'BlockCraft', description:'Game demo', icon:'game', category:'Games', width:720, height:460, content: renderGame },
    kate: { id:'kate', name:'Kate', description:'Advanced Text Editor', icon:'kate', category:'Development', width:760, height:480, content: () => renderGenericApp('Kate','Text editor') },
    kwrite: { id:'kwrite', name:'KWrite', description:'Text Editor', icon:'kwrite', category:'Development', width:680, height:430, content: () => renderGenericApp('KWrite','Text editor') },
    ark: { id:'ark', name:'Ark', description:'Archive Manager', icon:'ark', category:'Utilities', width:650, height:420, content: () => renderGenericApp('Ark','Archive manager') },
    kcalc: { id:'kcalc', name:'KCalc', description:'Calculator', icon:'kcalc', category:'Utilities', width:420, height:460, content: () => renderGenericApp('KCalc','Calculator') },
    spectacle: { id:'spectacle', name:'Spectacle', description:'Screenshot Utility', icon:'spectacle', category:'Graphics', width:650, height:430, content: () => renderGenericApp('Spectacle','Screenshot utility') },
    okular: { id:'okular', name:'Okular', description:'Document Viewer', icon:'okular', category:'Utilities', width:760, height:490, content: () => renderGenericApp('Okular','Document viewer') },
    gwenview: { id:'gwenview', name:'Gwenview', description:'Image Viewer', icon:'gwenview', category:'Graphics', width:760, height:490, content: () => renderGenericApp('Gwenview','Image viewer') },
    kdenlive: { id:'kdenlive', name:'Kdenlive', description:'Video Editor', icon:'kdenlive', category:'Multimedia', width:820, height:520, content: () => renderGenericApp('Kdenlive','Video editor') },
  };

  let pinnedApps = storageGetJson('flamewm-v8-pinned-apps', ['browser', 'files', 'terminal', 'code']).filter(id => applications[id]);
  const defaultDesktopItems = [
    { id:'home', label:'Home', icon:'home', app:'files', col:0, row:0, kind:'entry' },
    { id:'downloads', label:'Downloads', icon:'download', app:'files', col:0, row:1, kind:'entry' },
    { id:'projects', label:'Projects', icon:'folder', app:'code', col:0, row:2, kind:'entry' },
    { id:'browser-shortcut', label:'Firefox', icon:'browser', app:'browser', col:1, row:0, kind:'shortcut' },
    { id:'trash', label:'Trash', icon:'trash', app:null, col:0, row:4, kind:'trash' },
  ];
  let desktopItems = storageGetJson('flamewm-v8-desktop-items', defaultDesktopItems).filter(item => item && item.id);
  if (!desktopItems.some(item => item.id === 'trash')) desktopItems.push({...defaultDesktopItems.find(item => item.id === 'trash')});

  function boot() {
    applyAccent(state.accent, false);
    document.documentElement.dataset.iconTheme = state.iconTheme;
    applyWallpaper();
    updateWatermark();
    applyFontSettings(false);
    applyOverlayOpacities();
    applyTaskbarSettings();
    renderStickyNotes();
    renderStartButton();
    updateMediaButtonIcon();
    el.audioButton.innerHTML = svg('volume');
    el.wifiButton.innerHTML = svg('wifi');
    setTaskbarPosition(state.taskbarPosition, false);

    restoreDesktopPositions();
    renderDesktopIcons();
    updateTaskbar();
    renderPager();
    renderStartMenu();
    renderAudioPopup();
    renderMediaPopup();
    renderWifiPopup();
    renderClock();
    renderCalendar();
    attachGlobalEvents();

    // Seed two windows so focus, minimized state and task indicators are visible immediately.
    openApp('files', { x: 310, y: 80 });
    openApp('browser', { x: 80, y: 44, minimized: true });
  }

  function restoreDesktopPositions() {
    try {
      const saved = JSON.parse(storageGet('flamewm-v8-icon-layout', '{}'));
      desktopItems.forEach(item => {
        if (saved[item.id]) { item.col = saved[item.id].col; item.row = saved[item.id].row; }
      });
    } catch (_) {}
  }

  function persistDesktopPositions() {
    const out = {};
    desktopItems.forEach(i => out[i.id] = { col:i.col, row:i.row });
    storageSet('flamewm-v8-icon-layout', JSON.stringify(out));
    persistDesktopItems();
  }

  function persistDesktopItems() {
    storageSet('flamewm-v8-desktop-items', JSON.stringify(desktopItems));
    storageSet('flamewm-v8-trash-items', JSON.stringify(state.trashItems));
  }

  function gridMetrics() {
    const rect = el.grid.getBoundingClientRect();
    const cols = Math.max(1, Math.floor(Math.max(0, rect.width - ICON_W) / GRID_X) + 1);
    const rows = Math.max(1, Math.floor(Math.max(0, rect.height - ICON_H) / GRID_Y) + 1);
    return { rect, cols, rows };
  }

  function firstFreeGridCell(excludeId = null) {
    const { cols, rows } = gridMetrics();
    const occupied = new Set(desktopItems.filter(i => i.id !== excludeId).map(i => `${i.col}:${i.row}`));
    for (let col = 0; col < cols; col++) {
      for (let row = 0; row < rows; row++) {
        if (!occupied.has(`${col}:${row}`)) return { col, row };
      }
    }
    return { col: 0, row: 0 };
  }

  function normalizeDesktopPositions() {
    const { cols, rows } = gridMetrics();
    const occupied = new Set();
    for (const item of desktopItems) {
      let col = clamp(Number(item.col) || 0, 0, cols - 1);
      let row = clamp(Number(item.row) || 0, 0, rows - 1);
      if (occupied.has(`${col}:${row}`)) {
        const free = firstAvailableCell(occupied, cols, rows);
        col = free.col; row = free.row;
      }
      item.col = col; item.row = row;
      occupied.add(`${col}:${row}`);
    }
  }

  function firstAvailableCell(occupied, cols, rows) {
    for (let col = 0; col < cols; col++) {
      for (let row = 0; row < rows; row++) {
        if (!occupied.has(`${col}:${row}`)) return { col, row };
      }
    }
    return { col: 0, row: 0 };
  }

  function setDesktopSelection(ids) {
    state.selectedDesktopIcons = new Set(ids);
    document.querySelectorAll('.desktop-icon').forEach(node => node.classList.toggle('selected', state.selectedDesktopIcons.has(node.dataset.id)));
  }

  function renderDesktopIcons() {
    normalizeDesktopPositions();
    el.grid.innerHTML = '';
    desktopItems.forEach(item => {
      const node = document.createElement('div');
      node.className = `desktop-icon${state.selectedDesktopIcons.has(item.id) ? ' selected' : ''}`;
      node.dataset.id = item.id;
      node.style.left = `${item.col * GRID_X}px`;
      node.style.top = `${item.row * GRID_Y}px`;
      node.innerHTML = `${svg(item.icon)}<span class="desktop-label">${escapeHtml(item.label)}</span>`;
      if (item.id === 'trash') node.title = state.trashItems.length ? `${state.trashItems.length} item${state.trashItems.length===1?'':'s'} in Trash` : 'Trash is empty';
      node.addEventListener('dblclick', () => {
        if (item.id === 'trash') return toast(state.trashItems.length ? `Trash contains ${state.trashItems.length} item${state.trashItems.length===1?'':'s'}` : 'Trash is empty');
        if (item.app) openApp(item.app);
      });
      node.addEventListener('pointerdown', e => startDesktopIconDrag(e, item, node));
      node.addEventListener('contextmenu', e => {
        e.preventDefault(); e.stopPropagation();
        if (!state.selectedDesktopIcons.has(item.id)) setDesktopSelection([item.id]);
        openDesktopEntryMenu(item, node);
      });
      el.grid.appendChild(node);
    });
  }

  function startDesktopIconDrag(e, item, node) {
    if (e.button !== 0) return;
    e.stopPropagation();
    if (!state.selectedDesktopIcons.has(item.id)) setDesktopSelection([item.id]);
    const ids = [...state.selectedDesktopIcons].filter(id => desktopItems.some(i => i.id === id));
    const selectedNodes = ids.map(id => el.grid.querySelector(`.desktop-icon[data-id="${id}"]`)).filter(Boolean);
    const gridRect = el.grid.getBoundingClientRect();
    const starts = new Map(selectedNodes.map(n => {
      const model = desktopItems.find(i => i.id === n.dataset.id);
      return [n.dataset.id, { left: parseFloat(n.style.left)||0, top: parseFloat(n.style.top)||0, col:model.col, row:model.row }];
    }));
    const minLeft = Math.min(...[...starts.values()].map(v=>v.left));
    const minTop = Math.min(...[...starts.values()].map(v=>v.top));
    const maxRight = Math.max(...[...starts.values()].map(v=>v.left + ICON_W));
    const maxBottom = Math.max(...[...starts.values()].map(v=>v.top + ICON_H));
    let moved = false, lastDx = 0, lastDy = 0;
    try { node.setPointerCapture(e.pointerId); } catch (_) {}
    const move = ev => {
      let dx = ev.clientX - e.clientX, dy = ev.clientY - e.clientY;
      if (!moved && Math.abs(dx) + Math.abs(dy) < 5) return;
      moved = true;
      dx = clamp(dx, -minLeft, Math.max(-minLeft, gridRect.width - maxRight));
      dy = clamp(dy, -minTop, Math.max(-minTop, gridRect.height - maxBottom));
      lastDx = dx; lastDy = dy;
      selectedNodes.forEach(n => {
        const s = starts.get(n.dataset.id);
        n.classList.add('dragging');
        n.style.left = `${s.left + dx}px`;
        n.style.top = `${s.top + dy}px`;
      });
      const trash = el.grid.querySelector('.desktop-icon[data-id="trash"]');
      const canTrash = !ids.includes('trash') && ids.some(id => id !== 'trash');
      if (trash) trash.classList.toggle('trash-drop-target', canTrash && pointerOverDesktopItem(ev.clientX, ev.clientY, 'trash'));
    };
    const up = ev => {
      try { node.releasePointerCapture(ev.pointerId); } catch (_) {}
      node.removeEventListener('pointermove', move);
      node.removeEventListener('pointerup', up);
      selectedNodes.forEach(n => n.classList.remove('dragging'));
      el.grid.querySelector('.desktop-icon[data-id="trash"]')?.classList.remove('trash-drop-target');
      if (!moved) return;
      if (!ids.includes('trash') && pointerOverDesktopItem(ev.clientX, ev.clientY, 'trash')) {
        moveItemsToTrash(ids);
        return;
      }
      const proposedCol = Math.round(lastDx / GRID_X);
      const proposedRow = Math.round(lastDy / GRID_Y);
      const delta = findValidGroupDelta(ids, proposedCol, proposedRow);
      ids.forEach(id => {
        const model = desktopItems.find(i => i.id === id);
        const s = starts.get(id);
        if (model && s) { model.col = s.col + delta.col; model.row = s.row + delta.row; }
      });
      persistDesktopPositions();
      renderDesktopIcons();
    };
    node.addEventListener('pointermove', move);
    node.addEventListener('pointerup', up);
  }

  function findValidGroupDelta(ids, proposedCol, proposedRow) {
    const { cols, rows } = gridMetrics();
    const selected = new Set(ids);
    const models = ids.map(id => desktopItems.find(i => i.id === id)).filter(Boolean);
    const minCol = Math.min(...models.map(i=>i.col)), maxCol = Math.max(...models.map(i=>i.col));
    const minRow = Math.min(...models.map(i=>i.row)), maxRow = Math.max(...models.map(i=>i.row));
    proposedCol = clamp(proposedCol, -minCol, cols - 1 - maxCol);
    proposedRow = clamp(proposedRow, -minRow, rows - 1 - maxRow);
    const occupied = new Set(desktopItems.filter(i=>!selected.has(i.id)).map(i=>`${i.col}:${i.row}`));
    const valid = (dc,dr) => models.every(i => {
      const c=i.col+dc, r=i.row+dr;
      return c>=0 && c<cols && r>=0 && r<rows && !occupied.has(`${c}:${r}`);
    });
    if (valid(proposedCol, proposedRow)) return {col:proposedCol,row:proposedRow};
    const maxRadius=Math.max(cols,rows);
    for(let radius=1;radius<=maxRadius;radius++) {
      for(let dr=-radius;dr<=radius;dr++) for(let dc=-radius;dc<=radius;dc++) {
        const c=clamp(proposedCol+dc,-minCol,cols-1-maxCol), r=clamp(proposedRow+dr,-minRow,rows-1-maxRow);
        if(valid(c,r)) return {col:c,row:r};
      }
    }
    return {col:0,row:0};
  }

  function pointerOverDesktopItem(x, y, itemId) {
    const target = el.grid.querySelector(`.desktop-icon[data-id="${itemId}"]`);
    if (!target) return false;
    const r = target.getBoundingClientRect();
    return x >= r.left && x <= r.right && y >= r.top && y <= r.bottom;
  }

  function moveItemsToTrash(itemIds) {
    const ids = new Set(itemIds.filter(id => id !== 'trash'));
    const moved = desktopItems.filter(item => ids.has(item.id));
    if (!moved.length) return;
    desktopItems = desktopItems.filter(item => !ids.has(item.id));
    moved.forEach(item => state.trashItems.push({...item, trashedAt: Date.now()}));
    setDesktopSelection([]);
    persistDesktopItems();
    renderDesktopIcons();
    toast(moved.length === 1 ? `${moved[0].label} moved to Trash` : `${moved.length} items moved to Trash`);
  }

  function moveItemToTrash(itemId) { moveItemsToTrash([itemId]); }

  function deleteDesktopItem(itemId) {
    const index = desktopItems.findIndex(item => item.id === itemId);
    if (index < 0 || desktopItems[index].id === 'trash') return;
    const [item] = desktopItems.splice(index, 1);
    persistDesktopItems();
    renderDesktopIcons();
    toast(`${item.label} deleted`);
  }

  function createDesktopShortcut(itemId) {
    const source = desktopItems.find(item => item.id === itemId);
    if (!source || source.id === 'trash') return;
    const free = firstFreeGridCell();
    const shortcut = {
      id: `shortcut-${Date.now()}`,
      label: `${source.label} Shortcut`,
      icon: source.icon,
      app: source.app,
      col: free.col,
      row: free.row,
      kind: 'shortcut',
      shortcutOf: source.id,
    };
    desktopItems.push(shortcut);
    persistDesktopItems();
    renderDesktopIcons();
    toast(`Shortcut created for ${source.label}`);
  }

  function emptyTrash() {
    const count = state.trashItems.length;
    state.trashItems = [];
    persistDesktopItems();
    renderDesktopIcons();
    toast(count ? `Emptied ${count} item${count===1?'':'s'} from Trash` : 'Trash is already empty');
  }

  function findFreeGridCell(itemId, col, row) {
    const { cols, rows } = gridMetrics();
    col = clamp(col, 0, cols - 1); row = clamp(row, 0, rows - 1);
    const occupied = new Set(desktopItems.filter(i=>i.id!==itemId).map(i=>`${i.col}:${i.row}`));
    if (!occupied.has(`${col}:${row}`)) return {col,row};
    const maxRadius = Math.max(cols, rows);
    for (let radius=1; radius<=maxRadius; radius++) {
      for (let dy=-radius; dy<=radius; dy++) {
        for (let dx=-radius; dx<=radius; dx++) {
          const c=col+dx, r=row+dy;
          if (c>=0 && c<cols && r>=0 && r<rows && !occupied.has(`${c}:${r}`)) return {col:c,row:r};
        }
      }
    }
    const current = desktopItems.find(i=>i.id===itemId);
    return {col:clamp(current?.col ?? 0,0,cols-1),row:clamp(current?.row ?? 0,0,rows-1)};
  }

  function persistPinnedApps() {
    storageSet('flamewm-v8-pinned-apps', JSON.stringify(pinnedApps));
  }

  function persistTaskEntryOrder() {
    storageSet('flamewm-v8-task-entry-order', JSON.stringify(state.taskEntryOrder));
  }

  function currentTaskWindows() {
    return state.windows.filter(w => w.desktop === state.currentDesktop);
  }

  function buildTaskDescriptors() {
    const current = currentTaskWindows();
    const byApp = new Map();
    for (const w of current) {
      if (!byApp.has(w.appId)) byApp.set(w.appId, []);
      byApp.get(w.appId).push(w);
    }
    byApp.forEach(wins => wins.sort((a,b) => a.id - b.id));

    const descriptors = [];
    const represented = new Set();
    for (const appId of pinnedApps) {
      const wins = byApp.get(appId) || [];
      if (!wins.length) {
        descriptors.push({ key:`pin:${appId}`, appId, windowId:null, pinned:true, active:false });
        delete state.pinnedAnchorWindow[appId];
        continue;
      }
      let anchor = wins.find(w => w.id === state.pinnedAnchorWindow[appId]);
      if (!anchor) anchor = wins[0];
      state.pinnedAnchorWindow[appId] = anchor.id;
      descriptors.push({ key:`pin:${appId}`, appId, windowId:anchor.id, pinned:true, active:true });
      represented.add(anchor.id);
      for (const w of wins) {
        if (w.id === anchor.id) continue;
        descriptors.push({ key:`win:${w.id}`, appId, windowId:w.id, pinned:true, active:true });
        represented.add(w.id);
      }
    }
    for (const w of current) {
      if (represented.has(w.id)) continue;
      descriptors.push({ key:`win:${w.id}`, appId:w.appId, windowId:w.id, pinned:false, active:true });
    }

    const byKey = new Map(descriptors.map(d => [d.key, d]));
    state.taskEntryOrder = state.taskEntryOrder.filter(key => {
      if (key.startsWith('pin:')) return pinnedApps.includes(key.slice(4));
      if (key.startsWith('win:')) return Boolean(getWindow(Number(key.slice(4))));
      return false;
    });
    for (const d of descriptors) if (!state.taskEntryOrder.includes(d.key)) state.taskEntryOrder.push(d.key);
    persistTaskEntryOrder();
    return state.taskEntryOrder.map(key => byKey.get(key)).filter(Boolean);
  }

  function taskEntryWindow(descriptor) {
    return descriptor.windowId == null ? null : getWindow(descriptor.windowId);
  }

  function handleTaskEntryActivate(descriptor) {
    closePopovers();
    const w = taskEntryWindow(descriptor);
    if (!w) {
      openApp(descriptor.appId);
      return;
    }
    if (w.id === state.focusedWindowId && !w.minimized) {
      minimizeWindow(w.id);
      return;
    }
    w.minimized = false;
    renderWindow(w);
    focusWindow(w.id);
  }

  function pinAppForWindow(appId, windowId) {
    if (!applications[appId]) return;
    if (!pinnedApps.includes(appId)) pinnedApps.push(appId);
    persistPinnedApps();
    state.pinnedAnchorWindow[appId] = windowId;
    const winKey = `win:${windowId}`;
    const pinKey = `pin:${appId}`;
    const winIndex = state.taskEntryOrder.indexOf(winKey);
    const existingPin = state.taskEntryOrder.indexOf(pinKey);
    if (existingPin >= 0 && existingPin !== winIndex) state.taskEntryOrder.splice(existingPin, 1);
    const freshIndex = state.taskEntryOrder.indexOf(winKey);
    if (freshIndex >= 0) state.taskEntryOrder.splice(freshIndex, 1, pinKey);
    else if (!state.taskEntryOrder.includes(pinKey)) state.taskEntryOrder.push(pinKey);
    persistTaskEntryOrder();
    updateTaskbar();
    toast(`${applications[appId].name} pinned`);
  }

  function unpinApp(appId) {
    if (!pinnedApps.includes(appId)) return;
    const anchorWindowId = state.pinnedAnchorWindow[appId];
    pinnedApps = pinnedApps.filter(id => id !== appId);
    persistPinnedApps();
    const pinKey = `pin:${appId}`;
    const index = state.taskEntryOrder.indexOf(pinKey);
    if (index >= 0) {
      const anchor = anchorWindowId ? getWindow(anchorWindowId) : null;
      if (anchor && anchor.desktop === state.currentDesktop) state.taskEntryOrder.splice(index, 1, `win:${anchor.id}`);
      else state.taskEntryOrder.splice(index, 1);
    }
    delete state.pinnedAnchorWindow[appId];
    persistTaskEntryOrder();
    updateTaskbar();
    toast(`${applications[appId]?.name || 'Application'} unpinned`);
  }

  function mergeVisibleTaskOrder(visibleOrder) {
    const visibleSet = new Set(visibleOrder);
    const oldOrder = state.taskEntryOrder.slice();
    const firstVisible = oldOrder.findIndex(key => visibleSet.has(key));
    const hiddenOrder = oldOrder.filter(key => !visibleSet.has(key));
    const insertAt = firstVisible < 0
      ? hiddenOrder.length
      : oldOrder.slice(0, firstVisible).filter(key => !visibleSet.has(key)).length;
    hiddenOrder.splice(insertAt, 0, ...visibleOrder);
    state.taskEntryOrder = hiddenOrder;
  }

  function taskDropIndexForPointer(button, clientX, clientY) {
    const vertical = state.taskbarPosition === 'left' || state.taskbarPosition === 'right';
    const coord = vertical ? clientY : clientX;
    const siblings = [...el.tasks.querySelectorAll('.task-entry')].filter(n => n !== button);
    for (let i = 0; i < siblings.length; i++) {
      const r = siblings[i].getBoundingClientRect();
      const center = vertical ? r.top + r.height / 2 : r.left + r.width / 2;
      if (coord < center) return { index:i, siblings, vertical };
    }
    return { index:siblings.length, siblings, vertical };
  }

  function positionTaskDropIndicator(indicator, drop) {
    const { index, siblings, vertical } = drop;
    const taskRect = el.tasks.getBoundingClientRect();
    if (vertical) {
      const y = siblings.length === 0
        ? taskRect.top
        : index < siblings.length
          ? siblings[index].getBoundingClientRect().top
          : siblings[siblings.length - 1].getBoundingClientRect().bottom;
      indicator.style.left = `${taskRect.left + 5}px`;
      indicator.style.top = `${y - 1.5}px`;
      indicator.style.width = `${Math.max(10, taskRect.width - 10)}px`;
      indicator.style.height = '3px';
    } else {
      const x = siblings.length === 0
        ? taskRect.left
        : index < siblings.length
          ? siblings[index].getBoundingClientRect().left
          : siblings[siblings.length - 1].getBoundingClientRect().right;
      indicator.style.left = `${x - 1.5}px`;
      indicator.style.top = `${taskRect.top + 5}px`;
      indicator.style.width = '3px';
      indicator.style.height = `${Math.max(10, taskRect.height - 10)}px`;
    }
  }

  function wireTaskEntryDrag(button, descriptor) {
    button.addEventListener('pointerdown', e => {
      if (e.button !== 0) return;
      e.preventDefault();
      e.stopPropagation();

      const startX = e.clientX;
      const startY = e.clientY;
      const startRect = button.getBoundingClientRect();
      const pointerOffsetX = e.clientX - startRect.left;
      const pointerOffsetY = e.clientY - startRect.top;
      let dragging = false;
      let targetIndex = -1;
      let ghost = null;
      let indicator = null;

      state.taskDrag = { key:descriptor.key, pointerId:e.pointerId, targetIndex:-1 };
      try { button.setPointerCapture(e.pointerId); } catch (_) {}

      const startDragging = ev => {
        dragging = true;
        closePopovers();
        button.classList.add('dragging-source');
        button.setAttribute('aria-grabbed', 'true');

        ghost = button.cloneNode(true);
        ghost.classList.remove('dragging-source');
        ghost.classList.add('task-drag-ghost');
        ghost.removeAttribute('aria-grabbed');
        ghost.removeAttribute('data-task-key');
        ghost.style.width = `${startRect.width}px`;
        ghost.style.height = `${startRect.height}px`;
        document.body.appendChild(ghost);

        indicator = document.createElement('div');
        indicator.className = 'task-drop-indicator';
        document.body.appendChild(indicator);

        updateDragging(ev);
      };

      const updateDragging = ev => {
        if (!ghost || !indicator) return;
        ghost.style.left = `${ev.clientX - pointerOffsetX}px`;
        ghost.style.top = `${ev.clientY - pointerOffsetY}px`;
        const drop = taskDropIndexForPointer(button, ev.clientX, ev.clientY);
        targetIndex = drop.index;
        state.taskDrag.targetIndex = targetIndex;
        positionTaskDropIndicator(indicator, drop);
      };

      const move = ev => {
        if (ev.pointerId !== e.pointerId) return;
        if (!dragging && Math.hypot(ev.clientX - startX, ev.clientY - startY) < 6) return;
        if (!dragging) startDragging(ev);
        else updateDragging(ev);
      };

      const finish = ev => {
        if (ev.pointerId !== e.pointerId) return;
        window.removeEventListener('pointermove', move, true);
        window.removeEventListener('pointerup', finish, true);
        window.removeEventListener('pointercancel', finish, true);
        try {
          if (button.hasPointerCapture?.(e.pointerId)) button.releasePointerCapture(e.pointerId);
        } catch (_) {}

        button.classList.remove('dragging-source');
        button.removeAttribute('aria-grabbed');
        ghost?.remove();
        indicator?.remove();

        if (dragging && ev.type !== 'pointercancel') {
          const visibleOrder = [...el.tasks.querySelectorAll('.task-entry')]
            .map(n => n.dataset.taskKey)
            .filter(key => key !== descriptor.key);
          const boundedIndex = clamp(targetIndex < 0 ? visibleOrder.length : targetIndex, 0, visibleOrder.length);
          visibleOrder.splice(boundedIndex, 0, descriptor.key);
          mergeVisibleTaskOrder(visibleOrder);
          persistTaskEntryOrder();
          updateTaskbar();
        } else if (!dragging && ev.type !== 'pointercancel') {
          handleTaskEntryActivate(descriptor);
        }
        state.taskDrag = null;
      };

      window.addEventListener('pointermove', move, true);
      window.addEventListener('pointerup', finish, true);
      window.addEventListener('pointercancel', finish, true);
    });
  }

  function renderTaskEntries() {
    const descriptors = buildTaskDescriptors();
    el.tasks.innerHTML = '';
    for (const descriptor of descriptors) {
      const app = applications[descriptor.appId];
      if (!app) continue;
      const w = taskEntryWindow(descriptor);
      const button = document.createElement('button');
      button.className = `${w ? 'task-button' : 'pinned-button'} task-entry`;
      if (w) {
        button.classList.add('running');
        if (w.id === state.focusedWindowId && !w.minimized) button.classList.add('focused');
        if (w.minimized) button.classList.add('minimized');
      }
      button.dataset.taskKey = descriptor.key;
      button.dataset.appId = descriptor.appId;
      if (w) button.dataset.windowId = String(w.id);
      button.title = w ? `${app.name} — Window ${w.id}` : app.name;
      button.innerHTML = svg(app.icon);
      wireTaskEntryDrag(button, descriptor);
      button.addEventListener('contextmenu', e => {
        e.preventDefault(); e.stopPropagation();
        openTaskEntryMenu(descriptor, button);
      });
      el.tasks.appendChild(button);
    }
  }

  function workspaceGridPosition(index) {
    const columns = Math.max(1, Math.ceil(state.desktops / 2));
    if (index <= columns) return { row: 1, col: index, columns };
    return { row: 2, col: index - columns, columns };
  }

  function renderPager() {
    el.pager.innerHTML = '';
    el.pager.classList.toggle('hidden', state.desktops <= 1);
    if (state.desktops <= 1) return;
    const columns = Math.max(1, Math.ceil(state.desktops / 2));
    el.pager.style.setProperty('--workspace-columns', columns);
    const vertical = state.taskbarPosition === 'left' || state.taskbarPosition === 'right';
    for (let i=1; i<=state.desktops; i++) {
      const b = document.createElement('button');
      const pos = workspaceGridPosition(i);
      b.className = `workspace-button${i===state.currentDesktop?' active':''}`;
      b.textContent = i;
      b.title = `Desktop ${i}`;
      b.style.gridRow = String(vertical ? pos.col : pos.row);
      b.style.gridColumn = String(vertical ? pos.row : pos.col);
      b.addEventListener('click', () => switchDesktop(i));
      b.addEventListener('contextmenu', e => {
        e.preventDefault(); e.stopPropagation();
        openWorkspaceMenu(i, e.clientX, e.clientY);
      });
      el.pager.appendChild(b);
    }
  }

  function addDesktop(afterIndex = state.desktops) {
    if (state.desktops >= 18) return toast('Maximum 18 desktops in this prototype');
    afterIndex = clamp(afterIndex, 1, state.desktops);
    state.windows.forEach(w => { if (w.desktop > afterIndex) w.desktop++; });
    state.stickyNotes.forEach(n=>{if((n.desktop||1)>afterIndex)n.desktop++;}); persistStickyNotes();
    if (state.currentDesktop > afterIndex) state.currentDesktop++;
    state.desktops++;
    storageSet('flamewm-v8-desktops', state.desktops);
    renderPager();
    renderAllWindows();
    renderStickyNotes();
    updateTaskbar();
  }

  function removeDesktop(index) {
    if (state.desktops <= 1) return toast('At least one virtual desktop is required');
    index = clamp(index, 1, state.desktops);
    const target = index === 1 ? 1 : index - 1;
    state.windows.forEach(w => {
      if (w.desktop === index) w.desktop = target;
      else if (w.desktop > index) w.desktop--;
    });
    state.stickyNotes.forEach(n=>{const d=n.desktop||1;if(d===index)n.desktop=target;else if(d>index)n.desktop=d-1;}); persistStickyNotes();
    state.desktops--;
    if (state.currentDesktop === index) state.currentDesktop = target;
    else if (state.currentDesktop > index) state.currentDesktop--;
    storageSet('flamewm-v8-desktops', state.desktops);
    renderPager();
    renderAllWindows();
    renderStickyNotes();
    updateTaskbar();
  }

  function switchDesktop(index, moveWindowId = null) {
    if (index < 1 || index > state.desktops || index === state.currentDesktop) return;
    closePopovers();
    if (moveWindowId) {
      const w = getWindow(moveWindowId);
      if (w) w.desktop = index;
    }
    state.currentDesktop = index;
    state.focusedWindowId = null;
    renderPager();
    renderAllWindows();
    renderStickyNotes();
    updateTaskbar();
  }

  function switchDesktopDirectional(direction) {
    const here = workspaceGridPosition(state.currentDesktop);
    let target = state.currentDesktop;
    if (direction === 'left' && here.col > 1) target--;
    if (direction === 'right') {
      const candidate = state.currentDesktop + 1;
      if (candidate <= state.desktops && workspaceGridPosition(candidate).row === here.row) target = candidate;
    }
    if (direction === 'up' && here.row === 2) target = state.currentDesktop - here.columns;
    if (direction === 'down' && here.row === 1) {
      const candidate = state.currentDesktop + here.columns;
      if (candidate <= state.desktops) target = candidate;
    }
    if (target !== state.currentDesktop) switchDesktop(target);
  }

  function openWorkspaceMenu(index, x, y) {
    closePopovers();
    el.workspaceMenu.innerHTML = `
      <div class="menu-item" data-action="add">${svg('plus')}<span>Add desktop after ${index}</span></div>
      <div class="menu-item ${state.desktops<=1?'disabled':''}" data-action="remove">${svg('minus')}<span>Remove desktop ${index}</span></div>`;
    positionContextMenu(el.workspaceMenu, x, y);
    el.workspaceMenu.classList.remove('hidden');
    el.workspaceMenu.querySelector('[data-action="add"]').onclick = () => { addDesktop(index); closePopovers(); };
    el.workspaceMenu.querySelector('[data-action="remove"]').onclick = () => { removeDesktop(index); closePopovers(); };
  }

  function workArea() {
    const p = state.taskbarPosition;
    const W = window.innerWidth;
    const H = window.innerHeight;
    const panel=state.taskbarHeight;
    if (p === 'top') return { x:0, y:panel, width:W, height:Math.max(1,H-panel) };
    if (p === 'left') return { x:panel, y:0, width:Math.max(1,W-panel), height:H };
    if (p === 'right') return { x:0, y:0, width:Math.max(1,W-panel), height:H };
    return { x:0, y:0, width:W, height:Math.max(1,H-panel) };
  }

  function reflowWindowsForWorkArea() {
    const area = workArea();
    state.windows.forEach(w => {
      if (w.maximized || w.snap) return renderWindow(w);
      w.width = Math.min(w.width, area.width);
      w.height = Math.min(w.height, area.height);
      w.x = clamp(w.x, area.x - w.width + 80, area.x + area.width - 80);
      w.y = clamp(w.y, area.y, area.y + area.height - 31);
      renderWindow(w);
    });
  }

  function setTaskbarPosition(position, persist = true) {
    if (!['bottom','top','left','right'].includes(position)) position = 'bottom';
    state.taskbarPosition = position;
    el.desktop.dataset.taskbarPosition = position;
    if (persist) storageSet('flamewm-v8-taskbar-position', position);
    renderPager();
    renderDesktopIcons();
    renderStickyNotes();
    reflowWindowsForWorkArea();
    closePopovers();
  }

  function openApp(appId, opts = {}) {
    const app = applications[appId];
    if (!app) return;
    const cascade = state.windows.length % 6;
    const area = workArea();
    const width = Math.min(app.width, Math.max(260, area.width - 40));
    const height = Math.min(app.height, Math.max(160, area.height - 40));
    const x = clamp(opts.x ?? (area.x + 70 + cascade*28), area.x, Math.max(area.x, area.x + area.width - width));
    const y = clamp(opts.y ?? (area.y + 46 + cascade*24), area.y, Math.max(area.y, area.y + area.height - height));
    const w = {
      id: state.nextWindowId++, appId, title: app.name,
      desktop: state.currentDesktop,
      x, y, width, height,
      minimized: Boolean(opts.minimized),
      maximized: false,
      snap: null,
      restore: null,
      z: ++state.zCounter,
    };
    state.windows.push(w);
    renderWindow(w);
    if (!w.minimized) focusWindow(w.id);
    updateTaskbar();
  }

  function renderWindow(w) {
    const old = document.querySelector(`.window[data-window-id="${w.id}"]`);
    if (old) old.remove();
    if (w.desktop !== state.currentDesktop || w.minimized) return;
    const app = applications[w.appId];
    const node = document.createElement('section');
    node.className = `window${w.id===state.focusedWindowId?'':' inactive'}${w.maximized?' maximized':''}${w.snap?' snapped':''}`;
    node.dataset.windowId = w.id;
    node.dataset.appId = w.appId;
    applyWindowGeometry(node, w);
    node.style.zIndex = w.z;
    node.innerHTML = `
      <header class="window-titlebar">
        <div class="window-app-icon">${svg(app.icon)}</div>
        <div class="window-title">${w.title}</div>
        <div class="window-controls">
          <button class="window-control minimize" aria-label="Minimize">${svg('minus')}</button>
          <button class="window-control maximize" aria-label="Maximize">${svg(w.maximized || w.snap ? 'restore':'maximize')}</button>
          <button class="window-control close" aria-label="Close">${svg('close')}</button>
        </div>
      </header>
      <div class="window-content">${app.content()}</div>
      ${['n','s','e','w','ne','nw','se','sw'].map(d=>`<div class="resize-handle ${d}" data-dir="${d}"></div>`).join('')}`;
    el.windows.appendChild(node);
    wireWindow(node, w);
    if (w.appId === 'settings') wireSettingsWindow(node, w);
  }

  function renderAllWindows() {
    el.windows.innerHTML = '';
    state.windows.forEach(w => renderWindow(w));
  }

  function applyWindowGeometry(node, w) {
    const g = getSnapGeometry(w.snap || (w.maximized ? 'max':'custom'), w);
    node.style.left = `${g.x}px`;
    node.style.top = `${g.y}px`;
    node.style.width = `${g.width}px`;
    node.style.height = `${g.height}px`;
  }

  function getSnapGeometry(kind, w = null) {
    const area = workArea();
    const W = area.width, H = area.height;
    const halfW = Math.round(W/2), halfH = Math.round(H/2);
    switch(kind) {
      case 'max': return {x:area.x,y:area.y,width:W,height:H};
      case 'left': return {x:area.x,y:area.y,width:halfW,height:H};
      case 'right': return {x:area.x+halfW,y:area.y,width:W-halfW,height:H};
      case 'tl': return {x:area.x,y:area.y,width:halfW,height:halfH};
      case 'tr': return {x:area.x+halfW,y:area.y,width:W-halfW,height:halfH};
      case 'bl': return {x:area.x,y:area.y+halfH,width:halfW,height:H-halfH};
      case 'br': return {x:area.x+halfW,y:area.y+halfH,width:W-halfW,height:H-halfH};
      default: return {x:w?.x??(area.x+60),y:w?.y??(area.y+40),width:w?.width||640,height:w?.height||420};
    }
  }

  function wireWindow(node, w) {
    node.addEventListener('pointerdown', () => focusWindow(w.id));
    const titlebar = node.querySelector('.window-titlebar');
    titlebar.addEventListener('dblclick', e => {
      if (e.target.closest('.window-control')) return;
      toggleMaximize(w.id);
    });
    titlebar.addEventListener('pointerdown', e => {
      if (e.button !== 0 || e.target.closest('.window-control')) return;
      beginWindowDrag(e, w, node);
    });
    node.querySelector('.minimize').onclick = e => { e.stopPropagation(); minimizeWindow(w.id); };
    node.querySelector('.close').onclick = e => { e.stopPropagation(); closeWindow(w.id); };
    const maxBtn = node.querySelector('.maximize');
    maxBtn.onclick = e => { e.stopPropagation(); toggleMaximize(w.id); };
    node.querySelectorAll('.resize-handle').forEach(h => h.addEventListener('pointerdown', e => beginResize(e, w, node, h.dataset.dir)));
  }

  function focusWindow(id) {
    const w = getWindow(id); if (!w) return;
    w.minimized = false;
    w.z = ++state.zCounter;
    state.focusedWindowId = id;
    document.querySelectorAll('.window').forEach(n => n.classList.add('inactive'));
    const node = document.querySelector(`.window[data-window-id="${id}"]`);
    if (node) { node.classList.remove('inactive'); node.style.zIndex = w.z; }
    updateTaskbar();
  }
  function minimizeWindow(id) {
    const w=getWindow(id); if(!w)return; w.minimized=true;
    if(state.focusedWindowId===id) state.focusedWindowId=null;
    renderWindow(w); updateTaskbar(); focusTopVisibleWindow();
  }
  function closeWindow(id) {
    state.windows = state.windows.filter(w=>w.id!==id);
    document.querySelector(`.window[data-window-id="${id}"]`)?.remove();
    if(state.focusedWindowId===id) state.focusedWindowId=null;
    updateTaskbar(); focusTopVisibleWindow();
  }
  function focusTopVisibleWindow() {
    const top=state.windows.filter(w=>w.desktop===state.currentDesktop&&!w.minimized).sort((a,b)=>b.z-a.z)[0];
    if(top) focusWindow(top.id);
  }
  function toggleMaximize(id) {
    const w=getWindow(id); if(!w)return;
    if(w.maximized || w.snap) restoreWindow(w);
    else {
      saveRestore(w); w.maximized=true; w.snap=null;
    }
    renderWindow(w); focusWindow(w.id);
  }
  function saveRestore(w) { if (!w.restore) w.restore={x:w.x,y:w.y,width:w.width,height:w.height}; }
  function restoreWindow(w) {
    if(w.restore) Object.assign(w,w.restore);
    w.restore=null; w.maximized=false; w.snap=null;
  }

  function beginWindowDrag(e, w, node) {
    focusWindow(w.id);
    // Pull a snapped/maximized window back to floating while keeping the pointer near the same relative titlebar position.
    if (w.maximized || w.snap) {
      const before = node.getBoundingClientRect();
      const frac = clamp((e.clientX-before.left)/before.width, .12, .88);
      restoreWindow(w);
      const area = workArea();
      const newX = clamp(e.clientX - w.width*frac, area.x, Math.max(area.x, area.x+area.width-w.width));
      w.x = newX; w.y = clamp(e.clientY-15, area.y, Math.max(area.y, area.y+area.height-31));
      renderWindow(w);
      node = document.querySelector(`.window[data-window-id="${w.id}"]`);
    }
    const rect=node.getBoundingClientRect();
    state.drag={id:w.id, pointerId:e.pointerId, offX:e.clientX-rect.left, offY:e.clientY-rect.top};
    node.setPointerCapture(e.pointerId);
    const move=ev=>dragWindow(ev,w,node);
    const up=ev=>endWindowDrag(ev,w,node,move,up);
    node.addEventListener('pointermove',move);
    node.addEventListener('pointerup',up);
  }

  function dragWindow(e,w,node) {
    const area = workArea();
    const maxX=area.x+area.width-80;
    const maxY=area.y+area.height-31;
    w.x=clamp(e.clientX-state.drag.offX,area.x-w.width+80,maxX);
    w.y=clamp(e.clientY-state.drag.offY,area.y,maxY);
    w.maximized=false; w.snap=null;
    node.style.left=`${w.x}px`; node.style.top=`${w.y}px`;
    node.style.width=`${w.width}px`; node.style.height=`${w.height}px`;
    updateSnapCandidate(e.clientX,e.clientY);
    updateEdgeDesktopDwell(e.clientX,w);
  }

  function endWindowDrag(e,w,node,move,up) {
    try { node.releasePointerCapture(e.pointerId); } catch(_) {}
    node.removeEventListener('pointermove',move); node.removeEventListener('pointerup',up);
    clearEdgeDwell();
    if(state.snapCandidate && !state.drag?.switchedDesktop) applySnap(w,state.snapCandidate);
    hideSnapPreview();
    state.drag=null;
    renderWindow(w); focusWindow(w.id);
  }

  function updateSnapCandidate(x,y) {
    const area=workArea();
    const left=area.x, right=area.x+area.width, top=area.y, bottom=area.y+area.height;
    let kind=null;
    if (y <= top + SNAP_EDGE) {
      if (x <= left + SNAP_EDGE*2.3) kind='tl';
      else if (x >= right-SNAP_EDGE*2.3) kind='tr';
      else kind='max';
    } else if (y >= bottom-SNAP_EDGE) {
      if (x <= left+SNAP_EDGE*2.3) kind='bl';
      else if (x >= right-SNAP_EDGE*2.3) kind='br';
    } else if (x <= left+SNAP_EDGE) kind='left';
    else if (x >= right-SNAP_EDGE) kind='right';
    state.snapCandidate=kind;
    if(kind) showSnapPreview(kind); else hideSnapPreview();
  }

  function applySnap(w,kind) {
    saveRestore(w); w.snap=kind; w.maximized=kind==='max';
  }
  function showSnapPreview(kind) {
    const g=getSnapGeometry(kind);
    Object.assign(el.snap.style,{display:'block',left:`${g.x}px`,top:`${g.y}px`,width:`${g.width}px`,height:`${g.height}px`});
  }
  function hideSnapPreview() { el.snap.style.display='none'; state.snapCandidate=null; }

  function updateEdgeDesktopDwell(x,w) {
    // Corners remain reserved for quarter snapping; switch desktop only on the vertical middle of left/right edges.
    const y = w.y + 15;
    const area = workArea();
    const verticalSafe = y > area.y+40 && y < area.y+area.height-40;
    let dir=null;
    if(verticalSafe && x<=2 && state.currentDesktop>1) dir=-1;
    else if(verticalSafe && x>=window.innerWidth-2 && state.currentDesktop<state.desktops) dir=1;
    if(dir===state.edgeDirection) return;
    clearEdgeDwell();
    state.edgeDirection=dir;
    if(dir) {
      state.edgeTimer=setTimeout(()=>{
        const target=state.currentDesktop+dir;
        switchDesktopDuringDrag(target, w);
        toast(`Moved to Desktop ${target}`);
        clearEdgeDwell();
      },EDGE_DWELL_MS);
    }
  }
  function switchDesktopDuringDrag(target, draggedWindow) {
    if (target < 1 || target > state.desktops || target === state.currentDesktop) return;
    draggedWindow.desktop = target;
    state.currentDesktop = target;
    state.focusedWindowId = draggedWindow.id;
    if (state.drag) state.drag.switchedDesktop = true;
    // Preserve the live pointer-captured DOM node while replacing only the other desktop windows.
    const draggedNode = document.querySelector(`.window[data-window-id="${draggedWindow.id}"]`);
    el.windows.querySelectorAll('.window').forEach(node => { if (node !== draggedNode) node.remove(); });
    state.windows.filter(w => w.desktop === target && w.id !== draggedWindow.id && !w.minimized).forEach(w => renderWindow(w));
    if (draggedNode) { draggedNode.classList.remove('inactive'); draggedNode.style.zIndex = ++state.zCounter; draggedWindow.z = state.zCounter; }
    hideSnapPreview();
    renderPager();
    updateTaskbar();
  }
  function clearEdgeDwell(){ if(state.edgeTimer)clearTimeout(state.edgeTimer); state.edgeTimer=null; state.edgeDirection=null; }

  function beginResize(e,w,node,dir) {
    if(e.button!==0 || w.maximized || w.snap)return;
    e.stopPropagation(); focusWindow(w.id);
    const start={x:e.clientX,y:e.clientY,wx:w.x,wy:w.y,ww:w.width,wh:w.height};
    node.setPointerCapture(e.pointerId);
    const move=ev=>{
      const area = workArea();
      const dx=ev.clientX-start.x, dy=ev.clientY-start.y;
      let x=start.wx,y=start.wy,width=start.ww,height=start.wh;
      if(dir.includes('e')) width=Math.max(260,start.ww+dx);
      if(dir.includes('s')) height=Math.max(160,start.wh+dy);
      if(dir.includes('w')) { width=Math.max(260,start.ww-dx); x=start.wx+(start.ww-width); }
      if(dir.includes('n')) { height=Math.max(160,start.wh-dy); y=start.wy+(start.wh-height); }
      x=Math.max(area.x,x); y=Math.max(area.y,y);
      width=Math.min(width,area.x+area.width-x); height=Math.min(height,area.y+area.height-y);
      w.x=x;w.y=y;w.width=width;w.height=height;
      applyWindowGeometry(node,w);
    };
    const up=ev=>{try{node.releasePointerCapture(ev.pointerId)}catch(_){};node.removeEventListener('pointermove',move);node.removeEventListener('pointerup',up);};
    node.addEventListener('pointermove',move); node.addEventListener('pointerup',up);
  }


  function updateTaskbar() {
    el.taskbar.dataset.desktop = String(state.currentDesktop);
    renderTaskEntries();
  }

  function renderStartMenu(category = null, query = '') {
    const cats = ['Development','Games','Graphics','Internet','Multimedia','System','Utilities','Power / Session'];
    const categoryIcons = {
      Development:'code', Games:'game', Graphics:'graphics', Internet:'internet', Multimedia:'multimedia',
      System:'system', Utilities:'utility', 'Power / Session':'power'
    };
    const normalRows = () => cats.map(c=>`<button class="start-category" data-category="${c}">${svg(categoryIcons[c])}<span>${c}</span>${svg('chevron','arrow')}</button>`).join('');
    el.start.innerHTML = `
      <div class="start-root">
        <div class="start-content" id="start-content">${normalRows()}</div>
        <div class="start-search"><div class="search-wrap">${svg('search')}<input id="start-search-input" autocomplete="off" placeholder="Search…" value="${escapeHtml(query)}"></div></div>
      </div>
      <div class="start-submenu hidden" id="start-submenu"></div>`;

    const input = el.start.querySelector('#start-search-input');
    const content = el.start.querySelector('#start-content');
    const bindCategories = () => {
      el.start.querySelectorAll('[data-category]').forEach(b => {
        const show = () => {
          el.start.querySelectorAll('.start-category').forEach(x=>x.classList.toggle('active', x.dataset.category===b.dataset.category));
          renderStartSubmenu(b.dataset.category, '');
        };
        b.addEventListener('pointerenter', show);
        b.addEventListener('click', show);
      });
    };
    const showSearchResults = q => {
      const apps = Object.values(applications).filter(a => (a.name+' '+a.description+' '+a.category).toLowerCase().includes(q.toLowerCase()));
      content.innerHTML = `<div class="start-results-title">Applications</div>${apps.length ? apps.map(a=>`<button class="start-category start-search-result" data-search-app="${a.id}">${svg(a.icon)}<span>${escapeHtml(a.name)}</span></button>`).join('') : `<div class="start-empty">No applications</div>`}`;
      el.start.querySelector('#start-submenu').classList.add('hidden');
      content.querySelectorAll('[data-search-app]').forEach(b=>b.onclick=()=>{openApp(b.dataset.searchApp);closePopovers();});
    };
    bindCategories();
    input.addEventListener('input', () => {
      const q = input.value.trim();
      if (q) showSearchResults(q);
      else { content.innerHTML = normalRows(); bindCategories(); el.start.querySelector('#start-submenu').classList.add('hidden'); }
    });
    if (category) {
      const target=el.start.querySelector(`[data-category="${category}"]`); target?.dispatchEvent(new Event('click'));
    }
    if (query) showSearchResults(query);
  }

  function renderStartSubmenu(category, query) {
    const submenu = el.start.querySelector('#start-submenu');
    if (!submenu) return;
    let html = '';
    if (category === 'Power / Session' && !query) {
      html = `
        <button class="start-app" data-session="lock">${svg('lock')}<span>Lock Screen</span></button>
        <button class="start-app" data-session="logout">${svg('logout')}<span>Log Out</span></button>
        <button class="start-app" data-session="reboot">${svg('reboot')}<span>Restart</span></button>
        <button class="start-app" data-session="power">${svg('power')}<span>Shut Down</span></button>`;
    } else {
      let apps = Object.values(applications);
      if (query) apps = apps.filter(a => (a.name+' '+a.description+' '+a.category).toLowerCase().includes(query.toLowerCase()));
      else apps = apps.filter(a => a.category === category);
      html = apps.map(a=>`<button class="start-app" data-app="${a.id}">${svg(a.icon)}<span>${escapeHtml(a.name)}</span></button>`).join('');
      if (!html) html = `<div class="start-empty">No applications</div>`;
    }
    submenu.innerHTML = html;
    submenu.classList.remove('hidden');
    submenu.querySelectorAll('[data-app]').forEach(b=>b.onclick=()=>{openApp(b.dataset.app);closePopovers();});
    submenu.querySelectorAll('[data-session]').forEach(b=>b.onclick=()=>{toast(`${b.textContent.trim()} is simulated`);closePopovers();});
  }

  function renderAudioPopup() {
    el.audioPopup.innerHTML=`<div class="popup-title">Audio Volume</div><div class="popup-subtitle">Built-in Audio Analog Stereo</div>
      <div class="volume-line"><span id="popup-volume-icon">${svg(state.muted?'muted':'volume')}</span><input id="volume-slider" type="range" min="0" max="100" value="${state.volume}"><span id="volume-value">${state.muted?'Muted':state.volume+'%'}</span></div>
      <button class="tray-action" id="mute-toggle">${state.muted?'Unmute':'Mute'}</button>`;
    const slider=el.audioPopup.querySelector('#volume-slider');
    const commit = value => {
      state.volume=Math.max(0,Math.min(100,Math.round(value))); state.muted=false;
      slider.value=String(state.volume); storageSet('flamewm-v8-volume',state.volume);
      el.audioPopup.querySelector('#volume-value').textContent=`${state.volume}%`;
      el.audioPopup.querySelector('#popup-volume-icon').innerHTML=svg(state.volume===0?'muted':'volume'); updateTrayIcons();
    };
    slider.oninput=()=>commit(Number(slider.value));
    slider.addEventListener('pointerdown', e => {
      e.preventDefault(); try { slider.setPointerCapture(e.pointerId); } catch (_) {}
      const move = ev => { const r=slider.getBoundingClientRect(); commit(((ev.clientX-r.left)/Math.max(1,r.width))*100); };
      move(e);
      const up = ev => { try { slider.releasePointerCapture(ev.pointerId); } catch (_) {}; slider.removeEventListener('pointermove',move); slider.removeEventListener('pointerup',up); slider.removeEventListener('pointercancel',up); };
      slider.addEventListener('pointermove',move); slider.addEventListener('pointerup',up); slider.addEventListener('pointercancel',up);
    });
    el.audioPopup.querySelector('#mute-toggle').onclick=()=>{state.muted=!state.muted;renderAudioPopup();updateTrayIcons();};
  }
  function updateTrayIcons(){el.audioButton.innerHTML=svg(state.muted||state.volume===0?'muted':'volume');}
  function updateMediaButtonIcon(){ el.mediaButton.innerHTML = svg(state.mediaPlaying ? 'play' : 'pause'); el.mediaButton.title = state.mediaPlaying ? 'Playing' : 'Paused'; }

  function renderMediaPopup() {
    el.mediaPopup.innerHTML=`<div class="popup-title">Media Player</div><div class="media-now"><div class="album-art"></div><div class="media-meta"><div class="media-title">Neon Skyline</div><div class="media-artist">Prototype Player</div></div></div><div class="media-controls"><button class="media-control" data-media="prev">${svg('prev')}</button><button class="media-control" data-media="toggle">${svg(state.mediaPlaying?'pause':'play')}</button><button class="media-control" data-media="next">${svg('next')}</button></div>`;
    el.mediaPopup.querySelector('[data-media="toggle"]').onclick=()=>{state.mediaPlaying=!state.mediaPlaying;renderMediaPopup();updateMediaButtonIcon();};
    el.mediaPopup.querySelector('[data-media="prev"]').onclick=()=>toast('Previous track');
    el.mediaPopup.querySelector('[data-media="next"]').onclick=()=>toast('Next track');
  }

  function renderWifiPopup() {
    const nets=[['ArkNet 5G','Excellent'],['Studio','Good'],['Guest Network','Fair'],['CoffeeLab','Weak']];
    el.wifiPopup.innerHTML=`<div class="popup-title">Networks</div><div class="popup-subtitle">Wi-Fi is enabled</div>${nets.map(([n,s])=>`<div class="wifi-row ${state.wifiConnected===n?'connected':''}" data-network="${n}">${svg('wifi')}<div class="wifi-meta"><div class="wifi-name">${n}</div><div class="wifi-status">${state.wifiConnected===n?'Connected':s}</div></div>${state.wifiConnected===n?'✓':''}</div>`).join('')}<button class="tray-action" id="network-settings">Network details</button>`;
    el.wifiPopup.querySelectorAll('[data-network]').forEach(r=>r.onclick=()=>{state.wifiConnected=r.dataset.network;renderWifiPopup();toast(`Connected to ${state.wifiConnected}`);});
    el.wifiPopup.querySelector('#network-settings').onclick=()=>toast(`Connected: ${state.wifiConnected}`);
  }

  function renderClock() {
    const now=new Date();
    const time=now.toLocaleTimeString([], {hour:'2-digit',minute:'2-digit',hour12:false});
    const date=now.toLocaleDateString([], {day:'2-digit',month:'2-digit',year:'2-digit'});
    el.clockButton.innerHTML=`<span class="clock-time">${time}</span><span class="clock-date">${date}</span>`;
  }
  function renderCalendar() {
    const now=new Date(), year=now.getFullYear(), month=now.getMonth();
    const first=new Date(year,month,1), last=new Date(year,month+1,0);
    const mondayIndex=(first.getDay()+6)%7;
    const monthName=now.toLocaleDateString([], {month:'long'});
    const heads=['Mon','Tue','Wed','Thu','Fri','Sat','Sun'];
    let cells=heads.map(h=>`<div class="calendar-cell head">${h}</div>`).join('');
    const prevLast=new Date(year,month,0).getDate();
    for(let i=0;i<mondayIndex;i++)cells+=`<div class="calendar-cell dim">${prevLast-mondayIndex+i+1}</div>`;
    for(let d=1;d<=last.getDate();d++)cells+=`<div class="calendar-cell ${d===now.getDate()?'today':''}">${d}</div>`;
    const count=mondayIndex+last.getDate(); for(let n=1;n<=42-count;n++)cells+=`<div class="calendar-cell dim">${n}</div>`;
    el.clockPopup.innerHTML=`<div class="calendar-head"><span class="calendar-month">${monthName}</span><span class="calendar-year">${year}</span></div><div class="calendar-grid">${cells}</div>`;
  }

  function maximizeTaskWindow(w) {
    if (!w) return;
    if (!w.maximized) {
      saveRestore(w);
      w.maximized = true;
      w.snap = null;
    }
    w.minimized = false;
    renderWindow(w);
    focusWindow(w.id);
  }

  function openTaskEntryMenu(descriptor, anchor) {
    closePopovers();
    const app = applications[descriptor.appId];
    if (!app) return;
    const w = taskEntryWindow(descriptor);
    const isPinned = pinnedApps.includes(descriptor.appId);

    if (!w) {
      el.taskMenu.innerHTML = `
        <div class="menu-item" data-action="open">${svg(app.icon)}<span>Open</span></div>
        <div class="menu-item" data-action="unpin">${svg('minus')}<span>Unpin</span></div>`;
      el.taskMenu.classList.remove('hidden');
      positionTaskbarContextMenu(el.taskMenu, anchor);
      el.taskMenu.querySelector('[data-action="open"]').onclick = () => { openApp(descriptor.appId); closePopovers(); };
      el.taskMenu.querySelector('[data-action="unpin"]').onclick = () => { unpinApp(descriptor.appId); closePopovers(); };
      return;
    }

    const pinAction = isPinned
      ? `<div class="menu-item" data-action="unpin">${svg('minus')}<span>Unpin</span></div>`
      : `<div class="menu-item" data-action="pin">${svg('plus')}<span>Pin</span></div>`;
    const sizeAction = w.maximized && !w.minimized
      ? `<div class="menu-item" data-action="minimize">${svg('minus')}<span>Minimize</span></div>`
      : `<div class="menu-item" data-action="maximize">${svg('maximize')}<span>Maximize</span></div>`;
    el.taskMenu.innerHTML = `
      ${pinAction}
      <div class="menu-separator"></div>
      ${sizeAction}
      <div class="menu-item" data-action="close">${svg('close')}<span>Close</span></div>`;
    el.taskMenu.classList.remove('hidden');
    positionTaskbarContextMenu(el.taskMenu, anchor);
    el.taskMenu.querySelector('[data-action="pin"]')?.addEventListener('click', () => { pinAppForWindow(descriptor.appId, w.id); closePopovers(); });
    el.taskMenu.querySelector('[data-action="unpin"]')?.addEventListener('click', () => { unpinApp(descriptor.appId); closePopovers(); });
    el.taskMenu.querySelector('[data-action="maximize"]')?.addEventListener('click', () => { maximizeTaskWindow(w); closePopovers(); });
    el.taskMenu.querySelector('[data-action="minimize"]')?.addEventListener('click', () => { minimizeWindow(w.id); closePopovers(); });
    el.taskMenu.querySelector('[data-action="close"]').onclick = () => { closeWindow(w.id); closePopovers(); };
  }

  function openDesktopEntryMenu(item, anchor) {
    closePopovers();
    if (item.id === 'trash') {
      el.entryMenu.innerHTML = `
        <div class="menu-item" data-action="empty">${svg('trash')}<span>Empty Trash</span></div>`;
      el.entryMenu.classList.remove('hidden');
      positionDesktopEntryMenu(el.entryMenu, anchor);
      el.entryMenu.querySelector('[data-action="empty"]').onclick = () => { emptyTrash(); closePopovers(); };
      return;
    }
    el.entryMenu.innerHTML = `
      <div class="menu-item" data-action="shortcut">${svg('run')}<span>Create Shortcut</span></div>
      <div class="menu-separator"></div>
      <div class="menu-item" data-action="delete">${svg('close')}<span>Delete</span></div>`;
    el.entryMenu.classList.remove('hidden');
    positionDesktopEntryMenu(el.entryMenu, anchor);
    el.entryMenu.querySelector('[data-action="shortcut"]').onclick = () => { createDesktopShortcut(item.id); closePopovers(); };
    el.entryMenu.querySelector('[data-action="delete"]').onclick = () => { deleteDesktopItem(item.id); closePopovers(); };
  }

  function openDesktopMenu(x,y) {
    closePopovers();
    el.desktopMenu.innerHTML=`
      <div class="menu-item" data-action="terminal">${svg('terminal')}<span>Open Terminal</span></div>
      <div class="menu-item" data-action="folder">${svg('folderNew')}<span>Create New Folder</span></div>
      ${state.stickyNotesEnabled?`<div class="menu-item" data-action="sticky">${svg('sticky')}<span>New Sticky Note</span></div>`:''}
      <div class="menu-separator"></div>
      <div class="menu-item" data-action="settings">${svg('wallpaper')}<span>Desktop and Wallpaper…</span></div>`;
    positionContextMenu(el.desktopMenu,x,y); el.desktopMenu.classList.remove('hidden');
    el.desktopMenu.querySelector('[data-action="terminal"]').onclick=()=>{openApp('terminal');closePopovers();};
    el.desktopMenu.querySelector('[data-action="folder"]').onclick=()=>{createDemoFolder();closePopovers();};
    el.desktopMenu.querySelector('[data-action="sticky"]')?.addEventListener('click',()=>{createStickyNote(x,y);closePopovers();});
    el.desktopMenu.querySelector('[data-action="settings"]').onclick=()=>{openSettingsPage('desktop');closePopovers();};
  }

  function createDemoFolder() {
    const n = desktopItems.filter(i=>i.id.startsWith('new-folder')).length + 1;
    const free = firstFreeGridCell();
    desktopItems.push({id:`new-folder-${Date.now()}`,label:`New Folder ${n}`,icon:'folder',app:'files',col:free.col,row:free.row,kind:'entry'});
    renderDesktopIcons(); persistDesktopPositions();
  }

  function positionContextMenu(menu,x,y){
    const width = menu.offsetWidth || 200;
    const height = menu.offsetHeight || 150;
    menu.style.left=`${clamp(x,4,window.innerWidth-width-4)}px`;
    menu.style.top=`${clamp(y,4,window.innerHeight-height-4)}px`;
  }

  function positionDesktopEntryMenu(menu, anchor) {
    const r = anchor.getBoundingClientRect();
    const width = menu.offsetWidth || 200;
    const height = menu.offsetHeight || 100;
    menu.style.left = `${clamp(r.right + 5, 4, window.innerWidth-width-4)}px`;
    menu.style.top = `${clamp(r.top, 4, window.innerHeight-height-4)}px`;
  }

  function positionTaskbarContextMenu(menu, anchor) {
    const r = anchor.getBoundingClientRect();
    const bar = el.taskbar.getBoundingClientRect();
    const width = menu.offsetWidth || 200;
    const height = menu.offsetHeight || 80;
    const p = state.taskbarPosition;
    if (p === 'bottom') {
      menu.style.left = `${clamp(r.left,4,window.innerWidth-width-4)}px`;
      menu.style.top = `${Math.max(4, bar.top-height-4)}px`;
    } else if (p === 'top') {
      menu.style.left = `${clamp(r.left,4,window.innerWidth-width-4)}px`;
      menu.style.top = `${Math.min(window.innerHeight-height-4,bar.bottom+4)}px`;
    } else if (p === 'left') {
      menu.style.left = `${Math.min(window.innerWidth-width-4,bar.right+4)}px`;
      menu.style.top = `${clamp(r.top,4,window.innerHeight-height-4)}px`;
    } else {
      menu.style.left = `${Math.max(4,bar.left-width-4)}px`;
      menu.style.top = `${clamp(r.top,4,window.innerHeight-height-4)}px`;
    }
  }

  function positionPanelPopover(pop, button) {
    const r = button.getBoundingClientRect();
    const bar = el.taskbar.getBoundingClientRect();
    const width = pop.offsetWidth || 300;
    const height = pop.offsetHeight || 300;
    const p = state.taskbarPosition;
    if (p === 'bottom') {
      pop.style.left = `${clamp(r.left,4,window.innerWidth-width-4)}px`;
      pop.style.top = `${Math.max(4,bar.top-height-4)}px`;
    } else if (p === 'top') {
      pop.style.left = `${clamp(r.left,4,window.innerWidth-width-4)}px`;
      pop.style.top = `${Math.min(window.innerHeight-height-4,bar.bottom+4)}px`;
    } else if (p === 'left') {
      pop.style.left = `${Math.min(window.innerWidth-width-4,bar.right+4)}px`;
      pop.style.top = `${clamp(r.top,4,window.innerHeight-height-4)}px`;
    } else {
      pop.style.left = `${Math.max(4,bar.left-width-4)}px`;
      pop.style.top = `${clamp(r.top,4,window.innerHeight-height-4)}px`;
    }
    pop.style.right = 'auto'; pop.style.bottom = 'auto';
  }

  function togglePopover(pop, button) {
    const wasOpen=!pop.classList.contains('hidden'); closePopovers();
    if(!wasOpen){
      pop.classList.remove('hidden');
      if (button) { button.classList.add('active'); positionPanelPopover(pop, button); }
      state.openPopover=pop;
    }
  }
  function closePopovers(){
    [el.start,el.audioPopup,el.mediaPopup,el.wifiPopup,el.clockPopup,el.desktopMenu,el.workspaceMenu,el.taskMenu,el.entryMenu,el.taskbarMenu,el.stickyMenu].forEach(p=>p.classList.add('hidden'));
    [el.startButton,el.audioButton,el.mediaButton,el.wifiButton].forEach(b=>b.classList.remove('active'));
    state.openPopover=null;
  }

  function openTaskbarMenu(x,y) {
    closePopovers();
    el.taskbarMenu.innerHTML=`<div class="menu-item" data-action="newdesktop">${svg('plus')}<span>Create New Virtual Desktop</span></div><div class="menu-separator"></div><div class="menu-item" data-action="taskbarsettings">${svg('configure')}<span>Taskbar Settings…</span></div>`;
    positionContextMenu(el.taskbarMenu,x,y); el.taskbarMenu.classList.remove('hidden');
    el.taskbarMenu.querySelector('[data-action="newdesktop"]').onclick=()=>{addDesktop();closePopovers();};
    el.taskbarMenu.querySelector('[data-action="taskbarsettings"]').onclick=()=>{openSettingsPage('taskbar');closePopovers();};
  }

  function renderStartButton() {
    const icon = state.customStartIcon ? `<img class="breeze-img flamewm-start-icon" src="${state.customStartIcon}" alt="" draggable="false">` : svg('start');
    el.startButton.innerHTML=`${state.startButtonText?`<span class="start-custom-text">${escapeHtml(state.startButtonText)}</span>`:''}${icon}`;
  }

  function applyTaskbarSettings() {
    document.documentElement.style.setProperty('--panel-h', `${state.taskbarHeight}px`);
    const rgb=hexToRgb(state.taskbarColor);
    document.documentElement.style.setProperty('--taskbar-rgb', `${rgb.r},${rgb.g},${rgb.b}`);
    document.documentElement.style.setProperty('--taskbar-opacity', String(state.taskbarOpacity/100));
    renderStartButton();
  }

  function beginTaskbarDockDrag(e) {
    if (e.button !== 0) return;
    if (!(e.target === el.taskbar || e.target.classList.contains('taskbar-spacer'))) return;
    e.preventDefault();
    closePopovers();
    el.taskbar.classList.add('dragging-dock');
    try { el.taskbar.setPointerCapture(e.pointerId); } catch (_) {}
    const move = ev => updateTaskbarDockCandidate(ev.clientX, ev.clientY);
    const up = ev => {
      try { el.taskbar.releasePointerCapture(ev.pointerId); } catch (_) {}
      el.taskbar.removeEventListener('pointermove', move);
      el.taskbar.removeEventListener('pointerup', up);
      el.taskbar.classList.remove('dragging-dock');
      hideTaskbarDockPreview();
      const candidate = state.taskbarDockCandidate;
      state.taskbarDockCandidate = null;
      if (candidate && candidate !== state.taskbarPosition) setTaskbarPosition(candidate);
    };
    el.taskbar.addEventListener('pointermove', move);
    el.taskbar.addEventListener('pointerup', up);
  }

  function updateTaskbarDockCandidate(x, y) {
    const threshold = 74;
    const distances = [
      ['left', x], ['right', window.innerWidth-x], ['top', y], ['bottom', window.innerHeight-y]
    ].sort((a,b)=>a[1]-b[1]);
    const candidate = distances[0][1] <= threshold ? distances[0][0] : null;
    state.taskbarDockCandidate = candidate;
    if (!candidate) return hideTaskbarDockPreview();
    const st = el.dockPreview.style;
    st.display = 'block';
    if (candidate === 'top' || candidate === 'bottom') {
      st.left='0px'; st.width='100%'; st.height=`${state.taskbarHeight}px`;
      st.top=candidate==='top'?'0px':`${window.innerHeight-state.taskbarHeight}px`;
    } else {
      st.top='0px'; st.height='100%'; st.width=`${state.taskbarHeight}px`;
      st.left=candidate==='left'?'0px':`${window.innerWidth-state.taskbarHeight}px`;
    }
  }

  function hideTaskbarDockPreview() { el.dockPreview.style.display='none'; }

  function beginDesktopSelection(e) {
    if (e.button !== 0) return false;
    if (e.target.closest('.desktop-icon, .sticky-note, .window, #taskbar, .panel-popover, .context-menu')) return false;
    const g=el.grid.getBoundingClientRect();
    if (e.clientX < g.left || e.clientX > g.right || e.clientY < g.top || e.clientY > g.bottom) { setDesktopSelection([]); return false; }
    e.preventDefault();
    closePopovers();
    const startX=clamp(e.clientX,g.left,g.right), startY=clamp(e.clientY,g.top,g.bottom);
    let moved=false;
    try { el.desktop.setPointerCapture(e.pointerId); } catch (_) {}
    const draw = (x,y) => {
      const x1=clamp(Math.min(startX,x),g.left,g.right), x2=clamp(Math.max(startX,x),g.left,g.right);
      const y1=clamp(Math.min(startY,y),g.top,g.bottom), y2=clamp(Math.max(startY,y),g.top,g.bottom);
      el.selectionRect.classList.remove('hidden');
      el.selectionRect.style.left=`${x1}px`; el.selectionRect.style.top=`${y1}px`;
      el.selectionRect.style.width=`${x2-x1}px`; el.selectionRect.style.height=`${y2-y1}px`;
      const ids=[];
      document.querySelectorAll('.desktop-icon').forEach(node => {
        const r=node.getBoundingClientRect();
        if (r.right>=x1 && r.left<=x2 && r.bottom>=y1 && r.top<=y2) ids.push(node.dataset.id);
      });
      setDesktopSelection(ids);
    };
    const move=ev=>{ if(!moved && Math.abs(ev.clientX-startX)+Math.abs(ev.clientY-startY)<4)return; moved=true; draw(ev.clientX,ev.clientY); };
    const up=ev=>{
      try { el.desktop.releasePointerCapture(ev.pointerId); } catch (_) {}
      el.desktop.removeEventListener('pointermove',move); el.desktop.removeEventListener('pointerup',up);
      el.selectionRect.classList.add('hidden');
      if(!moved) setDesktopSelection([]);
    };
    el.desktop.addEventListener('pointermove',move); el.desktop.addEventListener('pointerup',up);
    return true;
  }

  function attachGlobalEvents() {
    el.startButton.onclick=e=>{e.stopPropagation();toggleStartMenu();};
    el.audioButton.onclick=e=>{e.stopPropagation();renderAudioPopup();togglePopover(el.audioPopup,el.audioButton);};
    el.mediaButton.onclick=e=>{e.stopPropagation();renderMediaPopup();togglePopover(el.mediaPopup,el.mediaButton);};
    el.wifiButton.onclick=e=>{e.stopPropagation();renderWifiPopup();togglePopover(el.wifiPopup,el.wifiButton);};
    el.clockButton.onclick=e=>{e.stopPropagation();renderCalendar();togglePopover(el.clockPopup,el.clockButton);};
    [el.start,el.audioPopup,el.mediaPopup,el.wifiPopup,el.clockPopup,el.desktopMenu,el.workspaceMenu,el.taskMenu,el.entryMenu,el.taskbarMenu,el.stickyMenu].forEach(p=>p.addEventListener('pointerdown',e=>e.stopPropagation()));
    el.taskbar.addEventListener('pointerdown', beginTaskbarDockDrag);
    el.taskbar.addEventListener('contextmenu', e=>{ if(e.target.closest('.task-button,.pinned-button,.workspace-button,.panel-button,.clock-button')) return; e.preventDefault(); e.stopPropagation(); openTaskbarMenu(e.clientX,e.clientY); });
    el.desktop.addEventListener('pointerdown',e=>{
      if (beginDesktopSelection(e)) return;
      if(!e.target.closest('#taskbar')&&!e.target.closest('.panel-popover')&&!e.target.closest('.context-menu')) closePopovers();
    });
    el.desktop.addEventListener('contextmenu',e=>{
      if(e.target.closest('#taskbar')||e.target.closest('.window')||e.target.closest('.sticky-note')||e.target.closest('.desktop-icon')||e.target.closest('.panel-popover')) return;
      e.preventDefault(); openDesktopMenu(e.clientX,e.clientY);
    });
    document.addEventListener('keydown', e => {
      if (state.recordingHotkey) {
        if (['Control','Alt','Shift','Meta'].includes(e.key)) return;
        e.preventDefault(); e.stopPropagation();
        if (e.key === 'Escape') { commitRecordedHotkey(''); return; }
        commitRecordedHotkey(normalizeHotkeyEvent(e));
        return;
      }
      if (e.metaKey && !['Meta','Shift','Control','Alt'].includes(e.key)) state.metaChordUsed = true;
      if (e.key === 'Escape') closePopovers();
      if (e.altKey && e.key === 'Tab') { e.preventDefault(); cycleWindows(); return; }
      if (hotkeyMatches(e, state.hotkeys.desktopLeft)) { e.preventDefault(); switchDesktopDirectional('left'); return; }
      if (hotkeyMatches(e, state.hotkeys.desktopRight)) { e.preventDefault(); switchDesktopDirectional('right'); return; }
      if (hotkeyMatches(e, state.hotkeys.desktopUp)) { e.preventDefault(); switchDesktopDirectional('up'); return; }
      if (hotkeyMatches(e, state.hotkeys.desktopDown)) { e.preventDefault(); switchDesktopDirectional('down'); return; }
      if (state.hotkeys.start !== 'Meta' && hotkeyMatches(e, state.hotkeys.start)) { e.preventDefault(); toggleStartMenu(); }
    });
    document.addEventListener('keyup', e => {
      if (state.recordingHotkey && ['Control','Alt','Shift','Meta'].includes(e.key)) {
        e.preventDefault(); commitRecordedHotkey(e.key); return;
      }
      if (e.key === 'Meta') {
        if (!state.metaChordUsed && state.hotkeys.start === 'Meta') toggleStartMenu();
        state.metaChordUsed = false;
      }
    });
    window.addEventListener('resize',()=>{
      setTaskbarPosition(state.taskbarPosition, false);
    });
    setInterval(renderClock,1000);
  }

  function toggleStartMenu() {
    const wasOpen = !el.start.classList.contains('hidden');
    closePopovers();
    if (wasOpen) return;
    renderStartMenu();
    el.start.dataset.desktop = String(state.currentDesktop);
    el.start.classList.remove('hidden');
    el.startButton.classList.add('active');
    positionPanelPopover(el.start, el.startButton);
    state.openPopover = el.start;
    setTimeout(()=>el.start.querySelector('input')?.focus(), 0);
  }

  function cycleWindows(){const wins=state.windows.filter(w=>w.desktop===state.currentDesktop).sort((a,b)=>b.z-a.z);if(!wins.length)return;const idx=wins.findIndex(w=>w.id===state.focusedWindowId);const next=wins[(idx+1)%wins.length];next.minimized=false;renderWindow(next);focusWindow(next.id);}

  function getWindow(id){return state.windows.find(w=>w.id===Number(id));}
  function clamp(v,min,max){return Math.max(min,Math.min(max,v));}
  function escapeHtml(s){return String(s).replace(/[&<>'"]/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;',"'":'&#39;','"':'&quot;'}[c]));}
  function toast(message){document.querySelectorAll('.toast').forEach(n=>n.remove());const n=document.createElement('div');n.className='toast';n.textContent=message;el.desktop.appendChild(n);setTimeout(()=>n.remove(),1700);}

  function hexToRgb(hex) {
    const value = String(hex).replace('#','').trim();
    if (!/^[0-9a-fA-F]{6}$/.test(value)) return {r:239,g:64,b:72};
    return {r:parseInt(value.slice(0,2),16),g:parseInt(value.slice(2,4),16),b:parseInt(value.slice(4,6),16)};
  }

  function applyAccent(color, persist = true) {
    const rgb = hexToRgb(color);
    state.accent = color;
    document.documentElement.style.setProperty('--accent', color);
    document.documentElement.style.setProperty('--accent-rgb', `${rgb.r}, ${rgb.g}, ${rgb.b}`);
    if (persist) storageSet('flamewm-v8-accent', color);
  }

  function applyWallpaper() {
    if (state.wallpaperData) {
      el.wallpaper.style.backgroundImage = `url("${state.wallpaperData.replace(/"/g,'%22')}")`;
      el.wallpaper.classList.add('custom-wallpaper');
    } else {
      el.wallpaper.style.backgroundImage = 'none';
      el.wallpaper.classList.remove('custom-wallpaper');
    }
  }

  function updateWatermark() {
    el.watermark.classList.toggle('hidden', !state.showWatermark);
  }

  function applyFontSettings(persist = true) {
    const safeFamily = String(state.fontFamily || 'IBM Plex Sans').replace(/["']/g,'');
    document.documentElement.style.setProperty('--ui-font-family', `"${safeFamily}", "IBM Plex Sans", "Noto Sans", sans-serif`);
    document.documentElement.style.setProperty('--font-size-offset', `${clamp(Number(state.fontSizeOffset)||0,-3,6)}px`);
    el.desktop.classList.toggle('font-bold', !!state.fontBold);
    if (persist) {
      storageSet('flamewm-v8-font-family', safeFamily);
      storageSet('flamewm-v8-font-bold', state.fontBold?'1':'0');
      storageSet('flamewm-v8-font-size-offset', state.fontSizeOffset);
    }
  }

  function applyOverlayOpacities() {
    document.documentElement.style.setProperty('--selection-opacity', String(state.selectionOpacity / 100));
    document.documentElement.style.setProperty('--snap-preview-opacity', String(state.snapPreviewOpacity / 100));
  }

  function persistDisplaySettings() {
    storageSet('flamewm-v8-display-settings', JSON.stringify(state.displaySettings));
  }

  function openSettingsPage(page) {
    state.settingsPage = page;
    const existing = state.windows.find(w=>w.appId==='settings' && w.desktop===state.currentDesktop);
    if (existing) {
      existing.minimized = false;
      renderWindow(existing);
      focusWindow(existing.id);
    } else openApp('settings');
  }

  function renderSettingsPage() {
    if (state.settingsPage === 'desktop') {
      return `<section class="settings-page"><h2>Desktop</h2><p class="settings-description">Wallpaper, desktop feedback and sticky notes.</p><div class="settings-card">
        <div class="setting-block"><div><strong>Background</strong><span>The FlameWM wallpaper is the default.</span></div><div class="setting-actions"><button class="settings-button" data-wallpaper-reset>FlameWM default</button><label class="settings-button file-button">Choose image<input data-wallpaper-file type="file" accept="image/*"></label></div></div>
        <div class="setting-block"><div><strong>Show FlameWM mark</strong></div><button class="toggle-button ${state.showWatermark?'on':''}" data-watermark-toggle aria-pressed="${state.showWatermark}"><span></span></button></div>
        <div class="setting-block"><div><strong>Sticky notes</strong><span>Disabling removes all notes.</span></div><button class="toggle-button ${state.stickyNotesEnabled?'on':''}" data-sticky-notes-toggle aria-pressed="${state.stickyNotesEnabled}"><span></span></button></div>
        <div class="setting-block vertical"><div><strong>Selection rectangle opacity</strong></div><div class="opacity-setting-row"><div class="opacity-demo selection-opacity-demo"><span></span></div><input type="range" min="0" max="60" step="1" value="${state.selectionOpacity}" data-selection-opacity><output>${state.selectionOpacity}%</output></div></div>
        <div class="setting-block vertical"><div><strong>Window layout preview opacity</strong></div><div class="opacity-setting-row"><div class="opacity-demo snap-opacity-demo"><span></span></div><input type="range" min="0" max="60" step="1" value="${state.snapPreviewOpacity}" data-snap-opacity><output>${state.snapPreviewOpacity}%</output></div></div>
      </div></section>`;
    }
    if (state.settingsPage === 'taskbar') {
      return `<section class="settings-page"><h2>Taskbar</h2><p class="settings-description">Customize the FlameWM taskbar.</p><div class="settings-card">
        <div class="setting-block"><div><strong>Color</strong></div><input class="custom-color-picker" type="color" value="${state.taskbarColor}" data-taskbar-color></div>
        <div class="setting-block vertical"><div><strong>Opacity</strong></div><div class="font-size-row"><input type="range" min="0" max="100" value="${state.taskbarOpacity}" data-taskbar-opacity><output>${state.taskbarOpacity}%</output></div></div>
        <div class="setting-block vertical"><div><strong>Height</strong></div><div class="font-size-row"><input type="range" min="34" max="72" value="${state.taskbarHeight}" data-taskbar-height><output>${state.taskbarHeight}px</output></div></div>
        <div class="setting-block"><div><strong>Start button text</strong></div><input class="settings-text-input" data-start-button-text value="${escapeHtml(state.startButtonText)}" placeholder="Optional"></div>
        <div class="setting-block"><div><strong>Start button icon</strong></div><div class="setting-actions"><label class="settings-button file-button">Choose icon<input data-start-icon-file type="file" accept="image/*,.svg"></label><button class="settings-button" data-start-icon-reset>Use FlameWM icon</button></div></div>
      </div></section>`;
    }
    if (state.settingsPage === 'displays') {
      const resolutionOptions = {'eDP-1':['1920x1080','1600x900','1366x768','1280x720'],'HDMI-1':['2560x1440','1920x1080','1600x900','1280x720']};
      const selectedId = state.displaySettings[state.selectedDisplayId] ? state.selectedDisplayId : Object.keys(state.displaySettings)[0];
      const selected = state.displaySettings[selectedId];
      const monitorButtons = Object.entries(state.displaySettings).map(([id,d]) => { const [rw,rh]=parseResolution(d.resolution); return `<button class="display-preview-monitor ${id===selectedId?'selected':''}" data-select-display="${id}" style="--display-aspect:${rw}/${rh};--display-ui-scale:${Number(d.scale)/100}"><span class="display-preview-screen"><span class="display-preview-window"></span><span class="display-preview-taskbar"><i></i><i></i><i></i></span></span><strong>${id}</strong><span>${d.resolution} · ${d.scale}%${d.primary?' · Primary':''}</span></button>`; }).join('');
      const [sw,sh]=parseResolution(selected.resolution); const options=resolutionOptions[selectedId]||[selected.resolution];
      return `<section class="settings-page"><h2>Displays</h2><p class="settings-description">Select a display to configure it.</p><div class="display-layout-preview">${monitorButtons}</div><div class="display-card selected-display-card"><div class="display-card-head">${svg('settingsDisplays')}<div><strong>${escapeHtml(selected.label)}</strong><span>${selectedId}${selected.primary?' · Primary':''}</span></div></div><div class="display-control"><label>Resolution</label><select data-display-resolution="${selectedId}">${options.map(v=>`<option ${v===selected.resolution?'selected':''}>${v}</option>`).join('')}</select></div><div class="display-control"><label>Scale</label><select data-display-scale="${selectedId}">${[100,125,150,175,200].map(v=>`<option value="${v}" ${v===Number(selected.scale)?'selected':''}>${v}%</option>`).join('')}</select></div><div class="display-simulation" style="aspect-ratio:${sw}/${sh};--sim-ui-scale:${Number(selected.scale)/100}"><div class="display-sim-shell"><div class="display-sim-window"><b></b><i></i><i></i></div><div class="display-sim-desktop-icon"></div><div class="display-sim-taskbar"><span></span><span></span><span></span><em></em></div></div></div><div class="display-simulation-note">${selected.resolution} · ${selected.scale}%</div></div></section>`;
    }
    if (state.settingsPage === 'fonts') {
      const families=['IBM Plex Sans','Noto Sans','Inter','Segoe UI','Ubuntu','Arial','sans-serif'];
      return `<section class="settings-page"><h2>Fonts</h2><p class="settings-description">Choose the desktop font.</p><div class="settings-card"><div class="setting-block"><div><strong>Font family</strong></div><select class="settings-select" data-font-family>${families.map(f=>`<option ${f===state.fontFamily?'selected':''}>${f}</option>`).join('')}</select></div><div class="setting-block"><div><strong>Bold globally</strong></div><button class="toggle-button ${state.fontBold?'on':''}" data-font-bold aria-pressed="${state.fontBold}"><span></span></button></div><div class="setting-block vertical"><div><strong>Size offset</strong></div><div class="font-size-row"><input type="range" min="-3" max="6" step="1" value="${state.fontSizeOffset}" data-font-size-offset><output>${state.fontSizeOffset>0?'+':''}${state.fontSizeOffset}px</output></div></div></div></section>`;
    }
    if (state.settingsPage === 'hotkeys') {
      const rows=[['start','Toggle Start menu'],['desktopLeft','Desktop left'],['desktopRight','Desktop right'],['desktopUp','Desktop up'],['desktopDown','Desktop down']];
      return `<section class="settings-page"><h2>Hotkeys</h2><p class="settings-description">Click a shortcut and press the new combination.</p><div class="settings-card hotkey-list">${rows.map(([key,label])=>`<div class="hotkey-row"><div class="hotkey-label"><strong>${label}</strong></div><div class="hotkey-controls">${state.hotkeys[key]!==DEFAULT_HOTKEYS[key]?`<button class="hotkey-revert" data-hotkey-reset="${key}" title="Restore default" aria-label="Restore ${label} default">${svg('undo')}</button>`:''}<button class="hotkey-recorder" data-hotkey="${key}">${formatHotkey(state.hotkeys[key])}</button></div></div>`).join('')}</div></section>`;
    }
    if (state.settingsPage === 'about') return `<section class="settings-page about-page"><img src="assets/flamewm.png" class="about-logo" alt="FlameWM"><p class="about-tagline">A lightweight desktop by ArkFlame Studios</p><div class="about-actions"><a class="about-link donate" href="https://paypal.me/LinsaFTW" target="_blank" rel="noopener noreferrer">${svg('paypal')}<span>Donate</span></a><a class="about-link" href="https://github.com/arkflame/flamewm" target="_blank" rel="noopener noreferrer">${svg('github')}<span>Source Code</span></a><a class="about-link" href="https://wm.arkflame.com" target="_blank" rel="noopener noreferrer">${svg('website')}<span>Website</span></a></div></section>`;
    const presets=['#ef4048','#ff7043','#f0b429','#55b86b','#3daee9','#7e66d7','#d84ca3'];
    const themes=['FlameWM Breeze (Built-in)','Breeze Dark','Breeze','Papirus','Adwaita','hicolor'];
    return `<section class="settings-page"><h2>Appearance</h2><p class="settings-description">Colors and icons.</p><div class="settings-card"><div class="setting-block vertical"><div><strong>Accent color</strong></div><div class="accent-palette">${presets.map(c=>`<button class="accent-swatch ${c.toLowerCase()===state.accent.toLowerCase()?'selected':''}" data-accent="${c}" style="--swatch:${c}" title="${c}"></button>`).join('')}</div></div><div class="setting-block"><div><strong>Custom color</strong></div><input class="custom-color-picker" data-custom-color type="color" value="${state.customAccentCandidate}"></div><div class="setting-block"><div><strong>Icon theme</strong></div><select class="settings-select" data-icon-theme>${themes.map(t=>`<option ${t===state.iconTheme?'selected':''}>${t}</option>`).join('')}</select></div></div></section>`;
  }

  function parseResolution(value) {
    const match = String(value || '').match(/^(\d+)x(\d+)$/);
    return match ? [Number(match[1]), Number(match[2])] : [16, 9];
  }

  function wireSettingsWindow(node, w) {
    node.querySelectorAll('[data-settings-page]').forEach(button => button.onclick = () => {
      state.settingsPage = button.dataset.settingsPage;
      renderWindow(w); focusWindow(w.id);
    });
    node.querySelectorAll('[data-accent]').forEach(button => button.onclick = () => {
      applyAccent(button.dataset.accent);
      renderWindow(w); focusWindow(w.id);
    });
    const custom = node.querySelector('[data-custom-color]');
    if (custom) custom.oninput = () => { state.customAccentCandidate = custom.value; storageSet('flamewm-v8-custom-accent', custom.value); applyAccent(custom.value); };
    node.querySelector('[data-icon-theme]')?.addEventListener('change', e=>{state.iconTheme=e.target.value;storageSet('flamewm-v8-icon-theme',state.iconTheme);document.documentElement.dataset.iconTheme=state.iconTheme;toast(`Icon theme: ${state.iconTheme}`);});
    node.querySelector('[data-wallpaper-reset]')?.addEventListener('click', () => {
      state.wallpaperData=DEFAULT_WALLPAPER; storageSet('flamewm-v8-wallpaper',DEFAULT_WALLPAPER); applyWallpaper(); toast('Background reset to FlameWM default');
    });
    const file = node.querySelector('[data-wallpaper-file]');
    if (file) file.onchange = () => {
      const chosen = file.files?.[0]; if (!chosen) return;
      const reader = new FileReader();
      reader.onload = () => {
        state.wallpaperData = String(reader.result || '');
        try { storageSet('flamewm-v8-wallpaper', state.wallpaperData); } catch (_) {}
        applyWallpaper(); toast('Wallpaper changed');
      };
      reader.readAsDataURL(chosen);
    };
    node.querySelector('[data-watermark-toggle]')?.addEventListener('click', () => {
      state.showWatermark=!state.showWatermark; storageSet('flamewm-v8-show-watermark',state.showWatermark?'1':'0'); updateWatermark(); renderWindow(w); focusWindow(w.id);
    });
    node.querySelectorAll('[data-select-display]').forEach(button => button.onclick = () => {
      state.selectedDisplayId = button.dataset.selectDisplay;
      storageSet('flamewm-v8-selected-display', state.selectedDisplayId);
      renderWindow(w); focusWindow(w.id);
    });
    node.querySelectorAll('[data-display-resolution]').forEach(select => select.onchange = () => {
      const id=select.dataset.displayResolution; state.displaySettings[id].resolution=select.value; persistDisplaySettings(); renderWindow(w); focusWindow(w.id);
    });
    node.querySelectorAll('[data-display-scale]').forEach(select => select.onchange = () => {
      const id=select.dataset.displayScale; state.displaySettings[id].scale=Number(select.value); persistDisplaySettings(); renderWindow(w); focusWindow(w.id);
    });
    node.querySelector('[data-selection-opacity]')?.addEventListener('input', e => {
      state.selectionOpacity = Number(e.target.value); storageSet('flamewm-v8-selection-opacity', state.selectionOpacity); applyOverlayOpacities(); e.target.nextElementSibling.textContent=`${state.selectionOpacity}%`;
    });
    node.querySelector('[data-snap-opacity]')?.addEventListener('input', e => {
      state.snapPreviewOpacity = Number(e.target.value); storageSet('flamewm-v8-snap-preview-opacity', state.snapPreviewOpacity); applyOverlayOpacities(); e.target.nextElementSibling.textContent=`${state.snapPreviewOpacity}%`;
    });
    node.querySelector('[data-font-family]')?.addEventListener('change', e => { state.fontFamily=e.target.value; applyFontSettings(); renderWindow(w); focusWindow(w.id); });
    node.querySelector('[data-font-bold]')?.addEventListener('click', () => { state.fontBold=!state.fontBold; applyFontSettings(); renderWindow(w); focusWindow(w.id); });
    node.querySelector('[data-font-size-offset]')?.addEventListener('input', e => { state.fontSizeOffset=Number(e.target.value); applyFontSettings(); e.target.nextElementSibling.textContent=`${state.fontSizeOffset>0?'+':''}${state.fontSizeOffset}px`; });
    node.querySelector('[data-sticky-notes-toggle]')?.addEventListener('click',()=>{ state.stickyNotesEnabled=!state.stickyNotesEnabled; storageSet('flamewm-v8-sticky-notes-enabled',state.stickyNotesEnabled?'1':'0'); if(!state.stickyNotesEnabled){state.stickyNotes=[];persistStickyNotes();renderStickyNotes();} renderWindow(w);focusWindow(w.id); });
    node.querySelector('[data-taskbar-color]')?.addEventListener('input',e=>{state.taskbarColor=e.target.value;storageSet('flamewm-v8-taskbar-color',state.taskbarColor);applyTaskbarSettings();});
    node.querySelector('[data-taskbar-opacity]')?.addEventListener('input',e=>{state.taskbarOpacity=Number(e.target.value);storageSet('flamewm-v8-taskbar-opacity',state.taskbarOpacity);applyTaskbarSettings();e.target.nextElementSibling.textContent=`${state.taskbarOpacity}%`;});
    const taskbarHeightInput=node.querySelector('[data-taskbar-height]');
    if(taskbarHeightInput){
      taskbarHeightInput.addEventListener('input',e=>{state.taskbarHeight=Number(e.target.value);storageSet('flamewm-v8-taskbar-height',state.taskbarHeight);applyTaskbarSettings();renderDesktopIcons();renderStickyNotes();e.target.nextElementSibling.textContent=`${state.taskbarHeight}px`;});
      const finalizeTaskbarHeight=()=>reflowWindowsForWorkArea();
      taskbarHeightInput.addEventListener('change',finalizeTaskbarHeight);
      taskbarHeightInput.addEventListener('pointerup',finalizeTaskbarHeight);
    }
    node.querySelector('[data-start-button-text]')?.addEventListener('input',e=>{state.startButtonText=e.target.value;storageSet('flamewm-v8-start-button-text',state.startButtonText);renderStartButton();});
    node.querySelector('[data-start-icon-reset]')?.addEventListener('click',()=>{state.customStartIcon='';storageSet('flamewm-v8-start-icon','');renderStartButton();renderWindow(w);focusWindow(w.id);});
    const startIconFile=node.querySelector('[data-start-icon-file]'); if(startIconFile) startIconFile.onchange=()=>{const f=startIconFile.files?.[0];if(!f)return;const r=new FileReader();r.onload=()=>{state.customStartIcon=String(r.result||'');storageSet('flamewm-v8-start-icon',state.customStartIcon);renderStartButton();};r.readAsDataURL(f);};
    node.querySelectorAll('[data-hotkey]').forEach(button => button.onclick = () => beginHotkeyRecording(button.dataset.hotkey, button));
    node.querySelectorAll('[data-hotkey-reset]').forEach(button => button.onclick = () => {
      const action = button.dataset.hotkeyReset;
      state.hotkeys[action] = DEFAULT_HOTKEYS[action] || '';
      storageSet('flamewm-v8-hotkeys',JSON.stringify(state.hotkeys));
      renderWindow(w); focusWindow(w.id);
    });
  }

  function beginHotkeyRecording(action, button) {
    state.recordingHotkey = { action, button, windowId: Number(button.closest('.window')?.dataset.windowId) };
    button.textContent = 'Press shortcut…';
    button.classList.add('recording');
  }

  function commitRecordedHotkey(shortcut) {
    if (!state.recordingHotkey) return;
    const { action, windowId } = state.recordingHotkey;
    state.hotkeys[action] = shortcut;
    storageSet('flamewm-v8-hotkeys', JSON.stringify(state.hotkeys));
    state.recordingHotkey = null;
    const w = getWindow(windowId);
    if (w) { renderWindow(w); focusWindow(w.id); }
  }

  function normalizeHotkeyEvent(e) {
    const parts=[];
    if(e.ctrlKey) parts.push('Ctrl');
    if(e.altKey) parts.push('Alt');
    if(e.shiftKey) parts.push('Shift');
    if(e.metaKey) parts.push('Meta');
    let key=e.key;
    if(key.length===1) key=key.toUpperCase();
    if(!['Control','Alt','Shift','Meta'].includes(key)) parts.push(key);
    return parts.join('+');
  }

  function hotkeyMatches(e, shortcut) {
    if (!shortcut || ['Meta','Control','Alt','Shift'].includes(shortcut)) return false;
    return normalizeHotkeyEvent(e) === shortcut;
  }

  function formatHotkey(shortcut) { return shortcut ? shortcut.replace('Meta','Win').replaceAll('+',' + ') : 'Not assigned'; }

  function persistStickyNotes(){ storageSet('flamewm-v8-sticky-notes',JSON.stringify(state.stickyNotes)); }
  function createStickyNote(x=120,y=120){
    if(!state.stickyNotesEnabled)return;
    const id=`note-${Date.now()}`; const g=el.grid.getBoundingClientRect();
    state.stickyNotes.push({id,text:'',desktop:state.currentDesktop,x:clamp(x,g.left,g.right-190),y:clamp(y,g.top,g.bottom-190),w:190,h:190,bg:'#ffe56b',fg:'#171717',size:15}); persistStickyNotes();renderStickyNotes();
    setTimeout(()=>el.stickyLayer.querySelector(`[data-note-id="${id}"] .sticky-note-editor`)?.focus(),0);
  }
  function renderStickyNotes(){
    el.stickyLayer.innerHTML=''; if(!state.stickyNotesEnabled)return;
    const g=el.grid.getBoundingClientRect();
    let normalized=false;
    state.stickyNotes.filter(note=>(note.desktop||1)===state.currentDesktop).forEach(note=>{
      const maxW=Math.max(120,g.width), maxH=Math.max(120,g.height);
      note.w=clamp(Number(note.w)||190,120,Math.min(520,maxW));
      note.h=clamp(Number(note.h)||190,120,Math.min(520,maxH));
      const nx=clamp(Number(note.x)||g.left,g.left,Math.max(g.left,g.right-note.w));
      const ny=clamp(Number(note.y)||g.top,g.top,Math.max(g.top,g.bottom-note.h));
      if(nx!==note.x||ny!==note.y){note.x=nx;note.y=ny;normalized=true;}
      const n=document.createElement('div'); n.className='sticky-note';n.dataset.noteId=note.id;n.style.cssText=`left:${note.x}px;top:${note.y}px;width:${note.w}px;height:${note.h}px;background:${note.bg};color:${note.fg};font-size:${note.size}px`;
      n.innerHTML=`<div class="sticky-note-editor" contenteditable="true" spellcheck="true">${escapeHtml(note.text)}</div>`;
      const editor=n.querySelector('.sticky-note-editor');
      editor.addEventListener('input',()=>{note.text=editor.innerText;persistStickyNotes();});
      n.addEventListener('contextmenu',e=>{e.preventDefault();e.stopPropagation();openStickyMenu(note,n,e.clientX,e.clientY);});
      n.addEventListener('pointerdown',e=>{
        e.stopPropagation();
        if(e.button!==0)return;
        const rect=n.getBoundingClientRect();
        const localX=e.clientX-rect.left, localY=e.clientY-rect.top;
        if(localX>rect.width-20&&localY>rect.height-20)return;
        if(localY>18)return;
        e.preventDefault();
        const sx=e.clientX,sy=e.clientY,ox=note.x,oy=note.y;
        const gridRect=el.grid.getBoundingClientRect();
        try{n.setPointerCapture(e.pointerId)}catch(_){};
        const mv=ev=>{note.x=clamp(ox+ev.clientX-sx,gridRect.left,Math.max(gridRect.left,gridRect.right-note.w));note.y=clamp(oy+ev.clientY-sy,gridRect.top,Math.max(gridRect.top,gridRect.bottom-note.h));n.style.left=`${note.x}px`;n.style.top=`${note.y}px`;};
        const up=ev=>{try{n.releasePointerCapture(ev.pointerId)}catch(_){};n.removeEventListener('pointermove',mv);n.removeEventListener('pointerup',up);const r=n.getBoundingClientRect();note.w=Math.round(r.width);note.h=Math.round(r.height);persistStickyNotes();};
        n.addEventListener('pointermove',mv);n.addEventListener('pointerup',up);
      });
      n.addEventListener('mouseup',()=>{const r=n.getBoundingClientRect();if(Math.abs(r.width-note.w)>1||Math.abs(r.height-note.h)>1){note.w=Math.round(r.width);note.h=Math.round(r.height);note.x=clamp(note.x,g.left,Math.max(g.left,g.right-note.w));note.y=clamp(note.y,g.top,Math.max(g.top,g.bottom-note.h));persistStickyNotes();renderStickyNotes();}});
      el.stickyLayer.appendChild(n);
    });
    if(normalized)persistStickyNotes();
  }
  function openStickyMenu(note,node,x,y){
    closePopovers(); el.stickyMenu.innerHTML=`<div class="menu-item" data-action="settings">${svg('configure')}<span>Settings</span></div><div class="menu-separator"></div><div class="menu-item" data-action="delete">${svg('close')}<span>Delete Note</span></div>`;positionContextMenu(el.stickyMenu,x,y);el.stickyMenu.classList.remove('hidden');
    el.stickyMenu.querySelector('[data-action="delete"]').onclick=()=>{state.stickyNotes=state.stickyNotes.filter(n=>n.id!==note.id);persistStickyNotes();renderStickyNotes();closePopovers();};
    el.stickyMenu.querySelector('[data-action="settings"]').onclick=()=>{el.stickyMenu.innerHTML=`<div class="sticky-settings"><label>Background<input type="color" data-note-bg value="${note.bg}"></label><label>Text<input type="color" data-note-fg value="${note.fg}"></label><label>Text size<input type="range" min="10" max="32" value="${note.size}" data-note-size><output>${note.size}px</output></label></div>`;const apply=()=>{node.style.background=note.bg;node.style.color=note.fg;node.style.fontSize=`${note.size}px`;persistStickyNotes();};el.stickyMenu.querySelector('[data-note-bg]').oninput=e=>{note.bg=e.target.value;apply();};el.stickyMenu.querySelector('[data-note-fg]').oninput=e=>{note.fg=e.target.value;apply();};el.stickyMenu.querySelector('[data-note-size]').oninput=e=>{note.size=Number(e.target.value);e.target.nextElementSibling.textContent=`${note.size}px`;apply();};};
  }

  function renderFiles(){return `<div class="file-window"><div class="file-menubar"><span>File</span><span>Edit</span><span>View</span><span>Go</span><span>Tools</span><span>Help</span></div><div class="file-toolbar"><span class="tool-icon">${svg('arrowLeft')}</span><span class="tool-icon">${svg('arrowRight')}</span><div class="location-bar">/home/juan/</div><span class="tool-icon">${svg('search')}</span><span class="tool-icon">${svg('menu')}</span></div><div class="file-body"><aside class="file-sidebar"><div class="file-sidebar-title">Places</div>${[['Home','home'],['Desktop','monitor'],['Documents','folder'],['Downloads','download'],['Music','music'],['Pictures','folder'],['Videos','folder']].map(([n,i],k)=>`<div class="file-sidebar-item ${k===0?'active':''}">${svg(i)}<span>${n}</span></div>`).join('')}<div class="file-sidebar-title">Remote</div><div class="file-sidebar-item">${svg('globe')}<span>Network</span></div><div class="file-sidebar-title">Devices</div><div class="file-sidebar-item">${svg('systemDisk')}<span>System Disk</span></div></aside><section class="file-grid">${['Downloads','Documents','Development','Cloud','Favorites'].map((n,i)=>`<div class="file-item">${svg(i===2?'code':'folder')}<span>${n}</span></div>`).join('')}</section></div></div>`;}
  function renderTerminal(){return `<div class="terminal"><div>Welcome to FlameWM Prototype v8.</div><div><span class="term-green">juan@flamewm</span>:<span class="term-blue">~</span>$ flamewm --about</div><br><div>Desktop: FlameWM</div><div>Base: IceWM 4.1.0</div><div>Prototype: v8</div><br><div><span class="term-green">juan@flamewm</span>:<span class="term-blue">~</span>$ <span class="terminal-cursor">_</span></div></div>`;}
  function renderBrowser(){return `<div class="browser-demo"><div class="browser-tabs"><div class="browser-tab">${svg('browser')}<span>New Tab</span></div></div><div class="browser-toolbar"><span class="tool-icon">${svg('arrowLeft')}</span><span class="tool-icon">${svg('arrowRight')}</span><div class="browser-address">https://wm.arkflame.com</div><span class="tool-icon">${svg('menu')}</span></div><div class="browser-page"><div class="browser-card"><img src="assets/flamewm.png" class="browser-flame-logo" alt="FlameWM"><h1>FlameWM Prototype v8</h1><p>Behavior reference for a finished, lightweight IceWM fork with Breeze-native iconography and FlameWM's red accent.</p></div></div></div>`;}
  function renderSettings(){
    const nav=[['appearance','settingsAppearance','Appearance'],['desktop','settingsDesktop','Desktop'],['taskbar','configure','Taskbar'],['displays','settingsDisplays','Displays'],['fonts','settingsFonts','Fonts'],['hotkeys','settingsHotkeys','Hotkeys'],['about','settingsAbout','About']];
    return `<div class="settings-demo"><aside class="settings-nav"><div class="settings-brand"><img src="assets/flamewm.png" alt="FlameWM"></div>${nav.map(([id,icon,label])=>`<button class="settings-item ${state.settingsPage===id?'active':''}" data-settings-page="${id}">${svg(icon)}<span>${label}</span></button>`).join('')}</aside><main class="settings-main">${renderSettingsPage()}</main></div>`;
  }
  function renderMusic(){return `<div style="height:100%;display:grid;grid-template-columns:150px 1fr;background:#202326"><aside style="padding:14px;background:#1b1e20;border-right:1px solid #111315"><div style="font-size:calc(18px + var(--font-size-offset));margin-bottom:16px">Elisa</div><div class="settings-item active">${svg('music')} Now Playing</div><div class="settings-item">${svg('folder')} Albums</div><div class="settings-item">${svg('media')} Tracks</div></aside><main style="display:flex;flex-direction:column;align-items:center;justify-content:center"><div class="album-art" style="width:120px;height:120px"></div><h2 style="margin:14px 0 3px">Neon Skyline</h2><div style="color:#aeb4ba">Prototype Player</div><div class="media-controls" style="margin-top:18px"><button class="media-control">${svg('prev')}</button><button class="media-control">${svg(state.mediaPlaying?'pause':'play')}</button><button class="media-control">${svg('next')}</button></div></main></div>`;}
  function renderCode(){return `<div style="height:100%;display:flex;background:#1e1e1e"><aside style="width:45px;background:#27292b;border-right:1px solid #161719;display:flex;flex-direction:column;align-items:center;padding-top:8px;gap:10px"><span class="tool-icon">${svg('files')}</span><span class="tool-icon">${svg('search')}</span><span class="tool-icon">${svg('code')}</span><span class="tool-icon">${svg('settings')}</span></aside><aside style="width:185px;background:#202224;border-right:1px solid #151617;padding:10px"><div style="font-size:calc(11px + var(--font-size-offset));color:#aeb4ba;margin-bottom:9px">EXPLORER</div><div>▾ flamewm-prototype-v8</div><div style="padding-left:12px;margin-top:7px">▣ index.html</div><div style="padding-left:12px;margin-top:5px"># styles.css</div><div style="padding-left:12px;margin-top:5px">JS app.js</div></aside><main class="terminal" style="flex:1;background:#1e1e1e"><span style="color:#569cd6">const</span> desktop = {<br>&nbsp;&nbsp;windowManager: <span style="color:#ce9178">'minimal'</span>,<br>&nbsp;&nbsp;theme: <span style="color:#ce9178">'flame-dark'</span>,<br>&nbsp;&nbsp;targetPssMiB: <span style="color:#b5cea8">40</span>,<br>};</main></div>`;}
  function renderGenericApp(name, description){return `<div class="generic-app"><div class="generic-app-card"><h2>${escapeHtml(name)}</h2><p>${escapeHtml(description)}</p><span>Real application entry included to exercise FlameWM Start categories, icons, window launching and task state.</span></div></div>`;}
  function renderGame(){return `<div style="height:100%;display:grid;place-items:center;background:linear-gradient(#62b7ed 0 55%,#78b653 55% 63%,#6b4b2f 63%)"><div style="width:210px;height:110px;background:#6aa848;border:8px solid #5b3b28;box-shadow:0 14px 0 #4d3427;display:grid;place-items:center;color:white;font-weight:700;font-size:calc(22px + var(--font-size-offset));text-shadow:2px 2px #314425">BLOCKCRAFT</div></div>`;}

  boot();
})();
