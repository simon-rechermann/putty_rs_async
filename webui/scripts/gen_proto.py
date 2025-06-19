#!/usr/bin/env python3
"""
Regenerate ES-module stubs into src/generated/ :

  • putty_interface_pb.ts        (messages + .d.ts)
  • putty_interface_connect.ts   (Promise client + .d.ts)
"""

from pathlib import Path
import subprocess, shutil, sys

ROOT   = Path(__file__).resolve().parents[1]          # …/webui
OUT    = ROOT / "src" / "generated"
PROTO  = ROOT.parent / "putty_grpc_server" / "proto" / "putty_interface.proto"
BIN    = ROOT / "node_modules" / ".bin"

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
    f"-I{PROTO.parent}",
    f"--es_out=target=ts,import_extension=.ts:{OUT}",
    f"--connect-es_out=target=ts,import_extension=.ts:{OUT}",
    str(PROTO),
]

print("→", " ".join(cmd))
subprocess.run(cmd, check=True)
print("✅  generated files in", OUT.relative_to(ROOT))
