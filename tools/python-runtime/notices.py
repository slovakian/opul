"""Retain the exact upstream notices for curl's statically linked dependencies."""
import hashlib
import tarfile


def notice_hashes(lock):
    if 'curl_licenses' not in lock or 'curl_additional_notices' not in lock:
        raise ValueError('Runtime lacks the complete native curl notice inventory. '
                         'Use its matching recipe revision for historical structural auditing.')
    return {**lock['curl_licenses']['files'],
            **{name: pin['sha256'] for name, pin in lock['curl_additional_notices'].items()}}


def install_notices(stage, archive, hashes):
    destination = stage / 'licenses/curl-impersonate'
    destination.mkdir(parents=True, exist_ok=True)
    with tarfile.open(archive) as tar:
        for name, digest in hashes.items():
            try:
                member = tar.getmember(name)
            except KeyError as error:
                raise ValueError(f'Missing native dependency notice: {name}') from error
            if not member.isfile():
                raise ValueError(f'Native dependency notice must be a regular file: {name}')
            with tar.extractfile(member) as source:
                data = source.read()
            if hashlib.sha256(data).hexdigest() != digest:
                raise ValueError(f'Native dependency notice checksum mismatch: {name}')
            (destination / name).write_bytes(data)
    verify_notices(stage, hashes)


def verify_notices(stage, hashes):
    destination = stage / 'licenses/curl-impersonate'
    for name, digest in hashes.items():
        path = destination / name
        if not path.is_file() or path.is_symlink():
            raise ValueError(f'Missing native dependency notice: {name}')
        if hashlib.sha256(path.read_bytes()).hexdigest() != digest:
            raise ValueError(f'Native dependency notice checksum mismatch: {name}')
