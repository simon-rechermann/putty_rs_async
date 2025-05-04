#!/usr/bin/env python3
import os, sys, threading, argparse, termios, tty
import grpc
import generated.putty_interface_pb2 as pb2
import generated.putty_interface_pb2_grpc as pb2_grpc


def enable_raw():
    fd = sys.stdin.fileno()
    old = termios.tcgetattr(fd)
    tty.setraw(fd, termios.TCSANOW)
    return old

def restore(old):
    termios.tcsetattr(sys.stdin.fileno(), termios.TCSADRAIN, old)


ap = argparse.ArgumentParser()
ap.add_argument('--target', default='127.0.0.1:50051')
sub = ap.add_subparsers(dest='proto', required=True)

ser = sub.add_parser('serial')
ser.add_argument('--port', default='/dev/pts/3')
ser.add_argument('--baud', type=int, default=115200)

ssh = sub.add_parser('ssh')
ssh.add_argument('--host', required=True)
ssh.add_argument('--port', type=int, default=22)
ssh.add_argument('--user', required=True)
ssh.add_argument('--password', default='')

args = ap.parse_args()

# ---- channel & stubs --------------------------------------------------
chan = grpc.insecure_channel(args.target)
stub = pb2_grpc.RemoteConnectionStub(chan)

if args.proto == 'serial':
    req = pb2.CreateRequest(serial=pb2.Serial(port=args.port, baud=args.baud))
else:
    req = pb2.CreateRequest(
        ssh=pb2.Ssh(host=args.host, port=args.port,
                    user=args.user,  password=args.password)
    )
conn_id = stub.CreateRemoteConnection(req).id
print(f'connected id={conn_id}; Ctrl+A x to quit')

stop_event = threading.Event()

# ---- reader thread ----------------------------------------------------
def reader():
    for chunk in stub.Read(pb2.ConnectionId(id=conn_id)):
        sys.stdout.buffer.write(chunk.data)
        sys.stdout.buffer.flush()
    stop_event.set()        # server closed stream

th = threading.Thread(target=reader, daemon=True)
th.start()

# ---- main loop: read stdin & write ------------------------------------
old_attr = enable_raw()
try:
    last_ctrl_a = False
    while not stop_event.is_set():
        ch = os.read(sys.stdin.fileno(), 1)
        if not ch:
            break
        b = ch[0]
        if b == 0x01:       # Ctrl+A
            last_ctrl_a = True
            continue
        if last_ctrl_a and b == ord('x'):
            stub.Stop(pb2.ConnectionId(id=conn_id))
            break
        last_ctrl_a = False
        stub.Write(pb2.WriteRequest(id=conn_id, data=ch))
finally:
    restore(old_attr)
    stop_event.set()
    th.join(timeout=1)
    print('\n[disconnected]')
