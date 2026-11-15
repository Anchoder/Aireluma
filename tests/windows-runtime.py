from pathlib import Path
import os
import shutil
import subprocess
import sys
import tempfile

root = Path(__file__).resolve().parents[1]
runtime = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else root / 'package/runtime'
toolchain = Path(sys.argv[2]).resolve() if len(sys.argv) > 2 else Path('C:/msys64/ucrt64')

with tempfile.TemporaryDirectory(prefix='aireluma-runtime-') as temporary:
    temporary = Path(temporary)
    for tool in ['gst-inspect-1.0.exe', 'gst-launch-1.0.exe']:
        shutil.copy2(toolchain / 'bin' / tool, temporary / tool)
    env = os.environ.copy()
    env['PATH'] = str(runtime) + os.pathsep + str(Path(env['WINDIR']) / 'System32')
    env['GST_PLUGIN_SYSTEM_PATH_1_0'] = str(runtime / 'plugins')
    env['GST_PLUGIN_PATH_1_0'] = ''
    env['GST_PLUGIN_SCANNER_1_0'] = str(runtime / 'gst-plugin-scanner.exe')
    env['GST_REGISTRY_1_0'] = str(temporary / 'registry.bin')

    def run(arguments):
        result = subprocess.run(arguments, cwd=temporary, env=env, capture_output=True, text=True, timeout=30)
        if result.returncode:
            raise RuntimeError(f'{arguments}: {result.stdout}\n{result.stderr}')
        if 'Failed to load plugin' in result.stderr:
            raise RuntimeError(result.stderr)

    for element in ['appsrc', 'decodebin', 'h264parse', 'h265parse', 'avdec_h264', 'avdec_h265',
                    'd3d12h264dec', 'd3d12h265dec', 'd3d12videosink', 'avdec_aac', 'avdec_alac',
                    'audioconvert', 'audioresample', 'volume', 'level', 'autoaudiosink',
                    'wasapi2sink', 'jpegdec']:
        run([str(temporary / 'gst-inspect-1.0.exe'), '--exists', element])
    subprocess.run([str(runtime / 'uxplay.exe'), '-h'], cwd=temporary, env=env,
                   stdout=subprocess.DEVNULL, check=True, timeout=10)
    for codec, encoder in [('h264', 'libx264'), ('h265', 'libx265')]:
        fixture = temporary / ('video.' + codec)
        subprocess.run([str(toolchain / 'bin/ffmpeg.exe'), '-v', 'error', '-f', 'lavfi',
                        '-i', 'testsrc2=size=640x360:rate=60', '-frames:v', '60', '-c:v', encoder,
                        '-preset', 'ultrafast', '-f', 'hevc' if codec == 'h265' else codec,
                        str(fixture)], stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, check=True, timeout=30)
        for decoder in ['avdec_' + codec, 'd3d12' + codec + 'dec']:
            run([str(temporary / 'gst-launch-1.0.exe'), '-q', 'filesrc', 'location=' + fixture.as_posix(),
                 '!', codec + 'parse', '!', decoder, '!', 'fakesink', 'sync=false'])
    test_plugins = temporary / 'test-plugins'
    test_plugins.mkdir()
    shutil.copy2(toolchain / 'lib/gstreamer-1.0/libgstaudiotestsrc.dll', test_plugins)
    env['GST_PLUGIN_PATH_1_0'] = str(test_plugins)
    for codec in ['aac', 'alac']:
        run([str(temporary / 'gst-launch-1.0.exe'), '-q', 'audiotestsrc', 'num-buffers=20',
             '!', 'audioconvert', '!', 'avenc_' + codec, '!', 'avdec_' + codec,
             '!', 'audioconvert', '!', 'audioresample', '!', 'volume', '!', 'level', '!', 'fakesink'])
    print('Isolated runtime: plugins, receiver, H.264/HEVC software and Direct3D12 decoding, AAC and ALAC audio passed.')
