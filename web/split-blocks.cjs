const fs = require('fs');
const path = require('path');

const appsCssPath = path.join(__dirname, 'src/styles/apps.css');
let content = fs.readFileSync(appsCssPath, 'utf8');

const blocks = {
    'themes-app': 'src/apps/theme-switcher/ThemeSwitcherApp.css',
    'explorer-app': 'src/apps/explorer/ExplorerApp.css',
    'text-reader-app': 'src/apps/text-reader/TextReaderApp.css',
    'pe-inspector': 'src/apps/pe-inspector/PeInspectorApp.css',
};

// Split by blocks. We can use a simple regex that matches blocks roughly.
// But simpler: just split by "\n\n" assuming standard formatting.
const chunks = content.split(/\n\s*\n/);

for (const chunk of chunks) {
    if (!chunk.trim()) continue;
    let target = null;
    
    if (chunk.includes('.themes-app') || chunk.includes('.theme-') || chunk.includes('.themes-')) target = blocks['themes-app'];
    else if (chunk.includes('.explorer-') || chunk.includes('.directory-') || chunk.includes('.file-') || chunk.includes('.address-bar') || chunk.includes('.toolbar') || chunk.includes('.sidebar')) target = blocks['explorer-app'];
    else if (chunk.includes('.text-reader-') || chunk.includes('.text-content') || chunk.includes('.text-area')) target = blocks['text-reader-app'];
    else if (chunk.includes('.pe-inspector') || chunk.includes('.pe-') || chunk.includes('.hex-')) target = blocks['pe-inspector'];
    
    if (target) {
        fs.appendFileSync(path.join(__dirname, target), chunk.trim() + '\n\n');
    } else {
        // If not matched, just put it back into apps.css
        fs.appendFileSync(appsCssPath + '.remaining', chunk.trim() + '\n\n');
    }
}

console.log('Split completed');
