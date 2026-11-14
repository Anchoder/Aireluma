import os
from pathlib import Path
import socket
import struct
import subprocess
import sys
import time

route = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
route.connect(('8.8.8.8', 80))
interface = route.getsockname()[0]
route.close()
listener = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
listener.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
listener.bind(('', 5353))
listener.setsockopt(socket.IPPROTO_IP, socket.IP_ADD_MEMBERSHIP, socket.inet_aton('224.0.0.251') + socket.inet_aton(interface))
listener.settimeout(0.25)
name = f'AirelumaProbe{os.getpid()}'
process = subprocess.Popen([str(Path(sys.argv[1]).resolve()), '-n', name, '-nh', '-vs', 'fakesink', '-as', 'fakesink', '-p', '35300'], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
times = []
first = None
deadline = time.monotonic() + 20
try:
    while time.monotonic() < deadline:
        if process.poll() is not None:
            raise RuntimeError(f'Receiver exited with {process.returncode}')
        try:
            packet, address = listener.recvfrom(65535)
            if name.encode() in packet and b'_airplay' in packet and len(packet) >= 12 and struct.unpack_from('!H', packet, 2)[0] & 0x8000:
                now = time.monotonic()
                if first is None:
                    first = now
                times.append(round(now - first, 3))
        except socket.timeout:
            pass
        if first is not None and time.monotonic() - first >= 10:
            break
finally:
    process.terminate()
    try:
        process.wait(timeout=5)
    except subprocess.TimeoutExpired:
        process.kill()
        process.wait()
    listener.close()

assert times, 'No initial AirPlay service announcement'
for low, high in [(0.75, 2.0), (2.75, 4.5), (6.75, 9.0)]:
    assert any(low <= value <= high for value in times), f'Missing startup announcement in {low}–{high}s: {times}'
assert not any(value > 9 for value in times), f'Unexpected periodic announcement after the startup burst: {times}'
print(f'AIRELUMA_MDNS_STARTUP_OK: {times}')
