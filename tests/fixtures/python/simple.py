import os
import sys
from pathlib import Path
from typing import List, Optional
from . import utils
from ..config import settings

def main() -> None:
    paths: List[Path] = []
    for root, dirs, files in os.walk('.'):
        for f in files:
            paths.append(Path(root) / f)
    print(paths)


def load_config(path: Optional[str] = None) -> dict:
    config_path = path or settings.DEFAULT_CONFIG
    return utils.load_json(config_path)


class FileScanner:
    def __init__(self, root: str) -> None:
        self.root = root

    def scan(self) -> List[str]:
        result = []
        for entry in os.listdir(self.root):
            result.append(entry)
        return result

    def filter(self, ext: str) -> List[str]:
        return [f for f in self.scan() if f.endswith(ext)]


if __name__ == '__main__':
    main()
