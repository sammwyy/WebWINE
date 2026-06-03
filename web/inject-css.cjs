const fs = require('fs');
const path = require('path');

const srcDir = path.join(__dirname, 'src');

function findAndInject(dir) {
    const files = fs.readdirSync(dir);
    for (const file of files) {
        const fullPath = path.join(dir, file);
        const stat = fs.statSync(fullPath);
        if (stat.isDirectory()) {
            findAndInject(fullPath);
        } else if (file.endsWith('.tsx')) {
            const cssName = file.replace('.tsx', '.css');
            const cssPath = path.join(dir, cssName);
            if (fs.existsSync(cssPath)) {
                let content = fs.readFileSync(fullPath, 'utf8');
                if (!content.includes(`import "./${cssName}"`)) {
                    content = `import "./${cssName}";\n` + content;
                    fs.writeFileSync(fullPath, content);
                    console.log(`Injected into ${file}`);
                }
            }
        }
    }
}

findAndInject(srcDir);
