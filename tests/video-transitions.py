from pathlib import Path
import shlex
import subprocess
import sys
import tempfile

root = Path(__file__).resolve().parents[1]
source = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else root / 'engine/uxplay'
flags = shlex.split(subprocess.check_output([
    'pkg-config', '--cflags', '--libs', 'gstreamer-1.0',
    'gstreamer-app-1.0', 'gstreamer-video-1.0'
], text=True))
with tempfile.TemporaryDirectory(prefix='aireluma-video-') as temporary:
    executable = Path(temporary) / ('transitions.exe' if sys.platform == 'win32' else 'transitions')
    subprocess.run([
        'gcc', '-std=gnu11', '-I' + str(source), str(root / 'tests/video-transitions.c'),
        str(source / 'lib/logger.c'), '-o', str(executable), *flags
    ], check=True)
    subprocess.run([str(executable)], check=True, timeout=30)
