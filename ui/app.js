// Satellite Setup's window. The work is done in Rust (src-tauri/src); this shows it and says it.
'use strict';

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const dialog = window.__TAURI__.dialog;

const $ = (id) => document.getElementById(id);
const files = {}; // option name -> chosen path
let devices = [];
let shownDevices = '';
let busy = false;

/**
 * Says `text` with the screen reader, without moving the focus. The text goes into a live region that is emptied
 * shortly after, so that the same message can be said again, and each message is a new element, which screen readers
 * notice more reliably than changed text.
 */
function announce(text) {
  const region = $('announcer');
  const message = document.createElement('div');
  message.textContent = text;
  region.appendChild(message);
  setTimeout(() => message.remove(), 10000);
}

function setStatus(text) {
  $('device-status').textContent = text;
}

/** What a device is, and whether setup can use it. */
function describe(device) {
  const kind = device.model.includes('25I') ? 'Orbit Speak' : device.model.includes('LZG403') ? 'Orbit Player' : device.model || 'A device';
  const how = device.connection === 'usb' ? 'connected by USB' : 'connected wirelessly';
  switch (device.state) {
    case 'unauthorized':
      return `${kind}, ${how}: waiting for you to allow this computer on it, in the "Allow USB debugging?" question`;
    case 'no permissions':
      return `A device ${how}, which this computer isn't allowed to use yet`;
    case 'offline':
      return `${kind}, ${how}: not answering`;
    case 'device':
      if (!device.orbit) return `${kind}, ${how}: not an Orbit Speak or Orbit Player`;
      if (!device.root) return `${kind}, ${how}: it doesn't give this computer root access, which setup needs`;
      return `${kind}, ${how}${device.set_up ? ', with Satellite set up' : ''}`;
    default:
      return `${kind}, ${how}: ${device.state}`;
  }
}

function usable(device) {
  return device && device.state === 'device' && device.orbit && device.root;
}

function selected() {
  return devices.find((d) => d.serial === $('device').value);
}

function updateButtons() {
  const device = selected();
  $('set-up').disabled = busy || !usable(device);
  $('undo').disabled = busy || !usable(device) || !device.set_up;
}

async function refreshDevices() {
  if (busy) return;
  try {
    devices = await invoke('devices');
  } catch (e) {
    setStatus(String(e));
    return;
  }
  // On Windows, an Orbit plugged in without a driver doesn't show up at all, so it is looked for separately.
  const needsDriver = devices.length === 0 && await invoke('usb_driver_needed').catch(() => false);
  const summary = JSON.stringify(devices) + needsDriver;
  if (summary === shownDevices) return;
  shownDevices = summary;
  const select = $('device');
  const previous = select.value;
  select.replaceChildren(...devices.map((device) => {
    const option = document.createElement('option');
    option.value = device.serial;
    option.textContent = describe(device);
    return option;
  }));
  // Keep the same device chosen; else the first that setup can use.
  const keep = devices.find((d) => d.serial === previous) || devices.find(usable) || devices[0];
  if (keep) select.value = keep.serial;
  select.disabled = devices.length === 0;
  $('allow-usb').hidden = !devices.some((d) => d.state === 'no permissions');
  $('usb-driver').hidden = !needsDriver;
  const text = needsDriver
    ? 'An Orbit is plugged in, but Windows needs a driver to use it. Choose Install the USB driver for the Orbit; ' +
      'Windows asks for permission first.'
    : devices.length === 0
      ? 'No Orbit is connected yet. Connect it with a USB cable.'
      : devices.length === 1 ? `Found: ${describe(devices[0])}.` : `Found ${devices.length} devices.`;
  setStatus(text);
  announce(text);
  updateButtons();
}

function setUpFileChoosers() {
  for (const row of document.querySelectorAll('.file')) {
    const { option, name, ext } = row.dataset;
    const choose = document.createElement('button');
    choose.type = 'button';
    choose.textContent = `Choose ${name}`;
    const chosen = document.createElement('span');
    chosen.className = 'chosen';
    chosen.id = `${option}-chosen`;
    chosen.textContent = 'Downloaded';
    choose.setAttribute('aria-describedby', chosen.id);
    const clear = document.createElement('button');
    clear.type = 'button';
    clear.textContent = `Don't use a file for the ${name}`;
    clear.hidden = true;
    choose.addEventListener('click', async () => {
      const path = await dialog.open({ title: `Choose the ${name}`, filters: [{ name, extensions: [ext] }] });
      if (!path) return;
      files[option] = path;
      chosen.textContent = path.split(/[\\/]/).pop();
      clear.hidden = false;
      announce(`${name}: ${chosen.textContent}`);
    });
    clear.addEventListener('click', () => {
      delete files[option];
      chosen.textContent = 'Downloaded';
      clear.hidden = true;
      choose.focus();
      announce(`The ${name} is downloaded.`);
    });
    row.append(choose, ' ', chosen, ' ', clear);
  }
}

function language() {
  const value = $('language').value;
  if (value === 'other') return $('other-language').value.trim() || null;
  return value || null;
}

function addProgress(progress) {
  const item = document.createElement('li');
  item.textContent = progress.text;
  if (progress.kind === 'stopped') item.className = 'stopped';
  $('progress').appendChild(item);
  announce(progress.text);
}

function setBusy(value) {
  busy = value;
  for (const id of ['device', 'pair', 'connect', 'google-play', 'turn-off-dnd', 'language', 'other-language']) {
    $(id).disabled = value || (id === 'device' && devices.length === 0);
  }
  updateButtons();
}

async function setUp() {
  const device = selected();
  if (!usable(device)) return;
  const tag = language();
  if ($('language').value === 'other' && !/^[A-Za-z]{2,3}(-[A-Za-z0-9]{2,8})*$/.test(tag || '')) {
    announce('Enter a language tag, such as fr-FR, or choose another language.');
    $('other-language').focus();
    return;
  }
  $('progress').replaceChildren();
  setBusy(true);
  await invoke('set_up', {
    serialno: device.serialno,
    options: {
      googlePlay: $('google-play').checked,
      turnOffDnd: $('turn-off-dnd').checked,
      locale: tag,
      launcherApk: files.launcherApk || null,
      hookApk: files.hookApk || null,
      backtalkApk: files.backtalkApk || null,
      lsposedZip: files.lsposedZip || null,
    },
  }).catch((e) => { addProgress({ kind: 'stopped', text: String(e) }); setBusy(false); });
}

async function undo() {
  const device = selected();
  if (!usable(device)) return;
  const sure = await dialog.ask(
    'This puts the Orbit back as it was before Satellite was set up, and removes the Satellite Launcher.',
    { title: 'Undo Satellite setup?', kind: 'warning', okLabel: 'Undo setup', cancelLabel: 'Cancel' });
  if (!sure) return;
  $('progress').replaceChildren();
  setBusy(true);
  await invoke('undo', { serialno: device.serialno })
    .catch((e) => { addProgress({ kind: 'stopped', text: String(e) }); setBusy(false); });
}

async function run(command, args, button) {
  button.disabled = true;
  try {
    announce(await invoke(command, args));
    shownDevices = '';
    await refreshDevices();
  } catch (e) {
    announce(String(e));
  } finally {
    button.disabled = busy;
  }
}

window.addEventListener('DOMContentLoaded', async () => {
  setUpFileChoosers();
  $('device').addEventListener('change', updateButtons);
  $('language').addEventListener('change', () => {
    $('other-language-row').hidden = $('language').value !== 'other';
  });
  $('set-up').addEventListener('click', setUp);
  $('undo').addEventListener('click', undo);
  $('pair').addEventListener('click', () =>
    run('pair', { address: $('pair-address').value, code: $('pair-code').value }, $('pair')));
  $('connect').addEventListener('click', () => run('connect', { address: $('connect-address').value }, $('connect')));
  $('allow-usb').addEventListener('click', () => run('allow_usb', {}, $('allow-usb')));
  $('usb-driver').addEventListener('click', () => run('install_usb_driver', {}, $('usb-driver')));
  await listen('progress', (event) => {
    addProgress(event.payload);
    if (event.payload.kind === 'done' || event.payload.kind === 'stopped') {
      setBusy(false);
      shownDevices = '';
      refreshDevices();
    }
  });
  try {
    await invoke('prepare');
  } catch (e) {
    setStatus(String(e));
    announce(String(e));
    return;
  }
  setStatus('Looking for the Orbit.');
  await refreshDevices();
  setInterval(refreshDevices, 3000);
});
