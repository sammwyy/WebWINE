const fs = require('fs');
const path = require('path');

const appsCssPath = path.join(__dirname, 'src/styles/apps.css');
const content = fs.readFileSync(appsCssPath, 'utf8');

const blocks = {
    'themes-app': 'src/apps/theme-switcher/ThemeSwitcherApp.module.css',
    'explorer-app': 'src/apps/explorer/ExplorerApp.module.css',
    'text-reader-app': 'src/apps/text-reader/TextReaderApp.module.css',
    'ctx-menu': 'src/components/shell/ContextMenu.module.css',
    'dialog-content': 'src/components/window/Dialog.module.css',
    'guest-win32': 'src/apps/guest-window/GuestWindowApp.module.css',
    'message-box': 'src/components/window/MessageBox.module.css',
    'process-console': 'src/apps/process-console/ProcessConsoleApp.module.css',
    'pe-inspector': 'src/apps/pe-inspector/PeInspectorApp.module.css',
    'properties-app': 'src/apps/properties/PropertiesApp.module.css'
};

const parts = content.split('/* ');

let remainingAppsCss = [];

for (let part of parts) {
    if (!part.trim()) continue;
    let header = '';
    let body = part;
    if (part.includes('*/')) {
        header = part.substring(0, part.indexOf('*/')).trim().toLowerCase();
        body = '/* ' + part;
    }
    
    let target = null;
    if (header.includes('themes app') || body.includes('.themes-app')) target = blocks['themes-app'];
    else if (header.includes('file explorer') || body.includes('.explorer-app')) target = blocks['explorer-app'];
    else if (header.includes('text reader') || body.includes('.text-reader-app')) target = blocks['text-reader-app'];
    else if (header.includes('context menu') || body.includes('.ctx-menu')) target = blocks['ctx-menu'];
    else if (header.includes('dialog content') || body.includes('.dialog-content')) target = blocks['dialog-content'];
    else if (header.includes('guest') || body.includes('.guest-content')) target = blocks['guest-win32'];
    else if (header.includes('message box') || body.includes('.message-box')) target = blocks['message-box'];
    else if (header.includes('cmd-style terminal') || body.includes('.process-console-app')) target = blocks['process-console'];
    else if (header.includes('pe inspector') || body.includes('.pe-inspector-app')) target = blocks['pe-inspector'];
    else if (header.includes('properties window') || body.includes('.properties-app')) target = blocks['properties-app'];

    if (header.includes('stderr') || header.includes('exit / crash') || header.includes('blinking') || header.includes('debug') || header.includes('echoed')) {
        target = blocks['process-console'];
    }
    
    if (header.includes('desktop icon cut state')) {
        fs.appendFileSync(path.join(__dirname, 'src/components/shell/DesktopIcon.module.css'), '\n' + body);
        continue;
    }

    if (target) {
        fs.appendFileSync(path.join(__dirname, target), '\n' + body);
    } else {
        remainingAppsCss.push(body);
    }
}

fs.writeFileSync(appsCssPath, remainingAppsCss.join('\n'));
console.log('Refactoring complete. remaining apps.css length: ' + remainingAppsCss.join('\n').length);
