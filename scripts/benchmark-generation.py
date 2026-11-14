#!/usr/bin/env python3
"""Release generation benchmark; run from the repository root, with no other builds.

python3 scripts/benchmark-generation.py --before /tmp/vados-before
"""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import statistics
import subprocess
import tempfile
import time

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--binary', default='target/release/vados')
parser.add_argument('--before', help='optional saved binary from before the change')
parser.add_argument('--runs', type=int, default=3)
args = parser.parse_args()
if args.runs < 1:
    parser.error('--runs must be positive')


def snapshot(root):
    return {str(p.relative_to(root)): hashlib.sha256(p.read_bytes()).digest()
            for p in root.rglob('*') if p.is_file()}


with tempfile.TemporaryDirectory(prefix='vados-benchmark-') as temporary:
    base = Path(temporary)
    pages, images = base / 'source', base / 'images'
    pages.mkdir()
    images.mkdir()
    (pages / 'main.json').write_text(json.dumps({
        'siteTitle': 'Benchmark', 'footerContent': '<p>Footer</p>',
        'jsFiles': [], 'cssFiles': []}))
    (pages / 'menu.json').write_text('{"mainMenu":[],"socials":[]}')
    (pages / 'page.json').write_text('{"title":"Home","content":"<p>Home</p>"}')
    for i in range(2000):
        page = pages / f'page-{i:04}'
        page.mkdir()
        (page / 'page.json').write_text(json.dumps({
            'title': f'Page {i}', 'content': 'content.md'}))
        (page / 'content.md').write_text(
            'Some Markdown with **bold** text.\n\n```rust\n'
            'fn main() {\n    println!("hello");\n}\n```\n')
    modes = []
    if args.before:
        modes.append(('before', args.before, []))
    modes += [('serial', args.binary, ['--jobs', '1']),
              ('parallel', args.binary, [])]
    expected = {}
    for label, binary, flags in modes:
        samples = {'bass cold': [], 'bass warm': [], '2000 pages': []}
        for _ in range(args.runs):
            for name, source, img in [
                    ('bass', Path('tests/fixtures/vados_bass/root'),
                     Path('tests/fixtures/vados_bass/imgroot')),
                    ('pages', pages, images)]:
                dest = base / 'output'
                shutil.rmtree(dest, ignore_errors=True)
                for run in range(2 if name == 'bass' else 1):
                    start = time.perf_counter()
                    subprocess.run([str(Path(binary).resolve()), 'generate',
                                    '--source', str(source), '--img-source', str(img),
                                    '--destination', str(dest), *flags],
                                   check=True, stdout=subprocess.DEVNULL)
                    elapsed = time.perf_counter() - start
                    key = ('bass cold' if run == 0 else 'bass warm') if name == 'bass' else '2000 pages'
                    samples[key].append(elapsed)
                    tree = snapshot(dest)
                    if name in expected:
                        assert tree == expected[name], f'{label}: different {name} output'
                    else:
                        expected[name] = tree
        print(label, {key: round(statistics.median(values), 3)
                      for key, values in samples.items()}, flush=True)
    print('All output trees match, including caches and manifests.')
