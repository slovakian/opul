"""Run with the TARGET interpreter, so bytecode matches the shipped Python."""
import compileall
from pathlib import Path
import shutil
import sys
import zipfile

prefix = Path(sys.argv[1])
stdlib = prefix / 'lib' / f'python{sys.version_info.major}.{sys.version_info.minor}'
site = stdlib / 'site-packages'
# curl_cffi explicitly uses certifi. OpenSSL's default Python contexts use macOS CAs.
(site / 'sitecustomize.py').write_text(
    "import os\n"
    "os.environ['SSL_CERT_FILE'] = '/etc/ssl/cert.pem'\n"
    "import certifi\n"
    "import curl_cffi.curl\n"
    "import curl_cffi.aio\n"
    # curl_cffi 0.17 otherwise prefers SSL_CERT_FILE and Python's system CAs.
    "curl_cffi.curl.DEFAULT_CACERT = certifi.where()\n"
    "curl_cffi.aio.DEFAULT_CACERT = certifi.where()\n")
for path in list(stdlib.rglob('*')):
    if path.name in {'test', 'tests', '__pycache__', 'idlelib', 'tkinter', 'turtledemo',
                      'ensurepip', 'pip', 'setuptools', 'wheel', 'SelfTest'} and path.is_dir():
        shutil.rmtree(path)
for path in stdlib.glob('lib-dynload/*'):
    if path.name.startswith(('_test', '_tkinter', 'xx')):
        path.unlink()
for name in ['include', 'share', 'lib/pkgconfig']:
    shutil.rmtree(prefix / name, ignore_errors=True)
# PBS's interpreter is statically linked; no extension uses libpython or Tcl/Tk.
for path in (prefix / 'lib').iterdir():
    if path != stdlib:
        if path.is_dir():
            shutil.rmtree(path)
        else:
            path.unlink()
for path in stdlib.glob('config-*'):
    shutil.rmtree(path)
for path in (prefix / 'bin').iterdir():
    if path.name not in {'python3', f'python{sys.version_info.major}.{sys.version_info.minor}', 'qjs'}:
        path.unlink()
# Legacy pyc names work inside zipimport and without source files. Stable filenames
# and checked hashes avoid host paths and timestamps leaking into bytecode.
if not compileall.compile_dir(stdlib, quiet=1, force=True, legacy=True,
                             stripdir=str(stdlib), prependdir='/opul/python',
                             invalidation_mode=__import__('py_compile').PycInvalidationMode.UNCHECKED_HASH):
    raise RuntimeError('Python bytecode compilation failed')
archive = prefix / 'lib' / f'python{sys.version_info.major}{sys.version_info.minor}.zip'
with zipfile.ZipFile(archive, 'w', compression=zipfile.ZIP_DEFLATED, compresslevel=9) as z:
    for path in sorted(stdlib.rglob('*')):
        if not path.is_file() or site in path.parents or 'lib-dynload' in path.parts:
            continue
        if path.suffix == '.pyc':
            info = zipfile.ZipInfo(path.relative_to(stdlib).as_posix(), (2026, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            z.writestr(info, path.read_bytes(), compresslevel=9)
for path in list(stdlib.iterdir()):
    if path.name not in {'site-packages', 'lib-dynload'}:
        if path.is_dir():
            shutil.rmtree(path)
        else:
            path.unlink()
for path in site.rglob('*.py'):
    path.unlink()
# Keep this directory even when CPython has no shared extension modules.
(stdlib / 'lib-dynload').mkdir(exist_ok=True)
