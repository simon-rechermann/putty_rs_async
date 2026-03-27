#!/usr/bin/env python3
"""
Regenerate ES-module stubs into src/generated/ :

  • putty_interface_pb.ts        (messages + .d.ts)
  • putty_interface_connect.ts   (Promise client + .d.ts)
"""

from pathlib import Path
import subprocess, shutil, sys

ROOT = Path(__file__).resolve().parents[1]  # …/webui
OUT = ROOT / "src" / "generated"
BIN = ROOT / "node_modules" / ".bin"

# Keep protoc paths relative to ROOT so Windows drive letters never end up in
# `--*_out=opts:path` arguments, where `:` is already used as a separator.
OUT_REL = Path("src") / "generated"
PROTO_REL = Path("..") / "putty_grpc_server" / "proto" / "putty_interface.proto"

def exe(name: str) -> str:            # adds .cmd on Windows
    suffix = ".cmd" if sys.platform == "win32" else ""
    return str(BIN / f"protoc-gen-{name}{suffix}")

# clean output dir
shutil.rmtree(OUT, ignore_errors=True)
OUT.mkdir(parents=True, exist_ok=True)

cmd = [
    "protoc",
    f"--plugin=protoc-gen-es={exe('es')}",
    f"--plugin=protoc-gen-connect-es={exe('connect-es')}",
    f"-I{PROTO_REL.parent.as_posix()}",
    f"--es_out=target=ts,import_extension=.ts:{OUT_REL.as_posix()}",
    f"--connect-es_out=target=ts,import_extension=.ts:{OUT_REL.as_posix()}",
    PROTO_REL.as_posix(),
]

print("protoc:", " ".join(cmd))
subprocess.run(cmd, check=True, cwd=ROOT)
print("generated files in", OUT.relative_to(ROOT))
