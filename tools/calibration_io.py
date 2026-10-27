"""Small, explicit study file contracts. Paths in study indexes are portable."""
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def load(path):
    return json.loads(Path(path).read_text())


def write(path, value):
    Path(path).write_text(json.dumps(value, indent=2, sort_keys=True) + '\n')


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def digest(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(',', ':')).encode()).hexdigest()


def require(condition, message):
    if not condition:
        raise ValueError(message)


def local(root, name):
    result = (Path(root) / name).resolve()
    require(result.is_relative_to(Path(root).resolve()), 'study path escapes its root')
    return result


def identity(task):
    return task['car'], task['case'], task['repeat']
