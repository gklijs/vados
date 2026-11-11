// Loaded only on pages displaying Markdown diagrams. Keep source untouched
// until rendering succeeds, including when the module cannot be downloaded.
(() => {
  const script = document.currentScript.dataset.mermaidScript;
  const blocks = [...document.querySelectorAll('.vados-mermaid > pre.mermaid')];
  if (!blocks.length) return;
  const preference = matchMedia('(prefers-color-scheme: dark)');
  const darkFor = (block) => {
    const scope = block.closest('[data-theme="light"], [data-theme="dark"], .theme-light, .theme-dark');
    if (!scope) return preference.matches;
    return scope.classList.contains('theme-dark') || scope.dataset.theme === 'dark';
  };
  import(script).then(({ default: mermaid }) => {
    let serial = 0;
    let pending = false;
    let running = false;
    async function render() {
      pending = true;
      if (running) return;
      running = true;
      try {
        // Serialise theme changes: Mermaid's configuration is global.
        while (pending) {
          pending = false;
          for (const block of blocks) {
            const dark = darkFor(block);
            const container = block.parentElement;
            let scratch;
            try {
              mermaid.initialize({
                startOnLoad: false,
                securityLevel: 'strict',
                suppressErrorRendering: true,
                theme: dark ? 'dark' : 'default',
                darkMode: dark,
                fontFamily: 'system-ui, sans-serif',
              });
              // A separate container lets a failed render leave the visible
              // source alone, and contains any error SVG Mermaid might create.
              scratch = document.createElement('div');
              scratch.className = 'vados-mermaid-rendered';
              scratch.style.cssText = 'position:absolute;visibility:hidden';
              container.append(scratch);
              const { svg, bindFunctions } = await mermaid.render(`vados-mermaid-${++serial}`, block.textContent, scratch);
              scratch.innerHTML = svg;
              bindFunctions?.(scratch);
              container.querySelectorAll('.vados-mermaid-rendered').forEach((old) => {
                if (old !== scratch) old.remove();
              });
              scratch.removeAttribute('style');
              block.hidden = true;
            } catch {
              container.querySelectorAll('.vados-mermaid-rendered').forEach((old) => old.remove());
              block.hidden = false;
            }
          }
        }
      } finally {
        running = false;
      }
    }
    preference.addEventListener('change', render);
    render();
  }).catch(() => { /* Source is already visible. */ });
})();
