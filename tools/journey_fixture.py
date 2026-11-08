"""Local CC0 media, HTTP tracker and real BitTorrent peer for packaged acceptance."""
import hashlib
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
import socketserver
import struct
import threading
import time
from urllib.parse import urlparse

ROOT = Path(__file__).resolve().parents[1]


def bencode(value):
    if isinstance(value, int):
        return b'i' + str(value).encode() + b'e'
    if isinstance(value, bytes):
        return str(len(value)).encode() + b':' + value
    if isinstance(value, dict):
        return b'd' + b''.join(bencode(k) + bencode(v) for k, v in sorted(value.items())) + b'e'
    raise ValueError('Unsupported fixture bencode value')


def receive(stream, length):
    result = b''
    while len(result) < length:
        chunk = stream.recv(length - len(result))
        if not chunk:
            raise EOFError()
        result += chunk
    return result


class JourneyFixture:
    def __init__(self, root, slow_media):
        self.gate = root / 'release-slow-transfer'
        self.media = (ROOT / 'tools/ffmpeg/fixtures/video.mp4').read_bytes()
        self.slow_media = slow_media.read_bytes()
        self.payloads = {}
        self.pieces_sent = []
        self.ranges = []
        self.stop = threading.Event()
        fixture = self

        class Peer(socketserver.BaseRequestHandler):
            def handle(self):
                self.request.settimeout(30)
                try:
                    handshake = receive(self.request, 68)
                    if handshake[:20] != b'\x13BitTorrent protocol':
                        return
                    info_hash = handshake[28:48]
                    payload = fixture.payloads[info_hash]
                    self.request.sendall(b'\x13BitTorrent protocol' + bytes(8) + info_hash
                                         + b'-OP0069-' + b'0' * 12)
                    self.request.sendall(struct.pack('>IBB', 2, 5, 0x80))
                    self.request.sendall(struct.pack('>IB', 1, 1))
                    while not fixture.stop.is_set():
                        size = struct.unpack('>I', receive(self.request, 4))[0]
                        if not size:
                            continue
                        if size > 65536:
                            return
                        message = receive(self.request, size)
                        if message[0] == 6:
                            index, begin, length = struct.unpack('>III', message[1:])
                            if index != 0 or length > 16384:
                                return
                            data = payload[begin:begin + length]
                            self.request.sendall(struct.pack('>IBII', len(data) + 9, 7, index, begin) + data)
                            fixture.pieces_sent.append({'info_hash': info_hash.hex(), 'bytes': len(data)})
                except (EOFError, OSError, KeyError):
                    return

        class Peers(socketserver.ThreadingTCPServer):
            daemon_threads = True
            allow_reuse_address = True

        self.peers = Peers(('127.0.0.1', 0), Peer)

        class Handler(BaseHTTPRequestHandler):
            def log_message(self, *_args):
                pass

            def do_HEAD(self):
                self.respond(False)

            def do_GET(self):
                self.respond(True)

            def respond(self, body):
                path = urlparse(self.path).path
                content_type = 'application/octet-stream'
                if path == '/playlist.html':
                    data = (b'<html><head><title>CC0 fixture playlist</title></head><body>'
                            b'<video controls src="/first.mp4"></video>'
                            b'<video controls src="/second.mp4"></video></body></html>')
                    content_type = 'text/html'
                elif path.startswith('/direct/'):
                    phase = Path(path).stem
                    data = f'Opul CC0 direct {phase}\n'.encode()
                elif path.startswith('/torrent/'):
                    phase = Path(path).stem
                    payload = (f'Opul CC0 torrent {phase}\n' * 32).encode()
                    info = {b'name': f'{phase}.bin'.encode(), b'length': len(payload),
                            b'piece length': 16384, b'pieces': hashlib.sha1(payload).digest()}
                    info_hash = hashlib.sha1(bencode(info)).digest()
                    fixture.payloads[info_hash] = payload
                    data = bencode({b'announce': f'{fixture.base}/announce'.encode(), b'info': info})
                    content_type = 'application/x-bittorrent'
                elif path == '/announce':
                    peer = b'\x7f\x00\x00\x01' + struct.pack('>H', fixture.peers.server_address[1])
                    data = bencode({b'interval': 1, b'min interval': 1, b'complete': 1,
                                    b'incomplete': 0, b'peers': peer})
                elif path in {'/single.mp4', '/first.mp4', '/second.mp4', '/success.mp4'}:
                    data = fixture.media
                    content_type = 'video/mp4'
                elif path == '/slow.mp4':
                    data = fixture.slow_media
                    content_type = 'video/mp4'
                else:
                    self.send_error(404)
                    return
                start = 0
                if self.headers.get('Range', '').startswith('bytes='):
                    start = int(self.headers['Range'][6:].split('-')[0])
                    fixture.ranges.append({'path': path, 'start': start})
                self.send_response(206 if start else 200)
                self.send_header('Content-Type', content_type)
                self.send_header('Content-Length', str(len(data) - start))
                self.send_header('Accept-Ranges', 'bytes')
                if start:
                    self.send_header('Content-Range', f'bytes {start}-{len(data) - 1}/{len(data)}')
                self.end_headers()
                if not body:
                    return
                try:
                    if path == '/slow.mp4' and not fixture.gate.exists():
                        end = min(start + 131072, len(data))
                        self.wfile.write(data[start:end])
                        self.wfile.flush()
                        start = end
                        while not fixture.gate.exists() and not fixture.stop.is_set():
                            time.sleep(0.05)
                    self.wfile.write(data[start:])
                except (BrokenPipeError, ConnectionResetError):
                    pass

        self.http = ThreadingHTTPServer(('127.0.0.1', 0), Handler)
        self.base = f'http://127.0.0.1:{self.http.server_port}'
        for server in [self.peers, self.http]:
            threading.Thread(target=server.serve_forever, daemon=True).start()

    def close(self):
        self.stop.set()
        self.gate.touch()
        for server in [self.http, self.peers]:
            server.shutdown()
            server.server_close()
