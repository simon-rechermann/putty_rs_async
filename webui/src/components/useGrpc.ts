import { useRef, useState } from "react";
import { createGrpcWebTransport } from "@connectrpc/connect-web";
import { createPromiseClient }    from "@connectrpc/connect";

import {
  CreateRequest, Serial, Ssh, WriteRequest,
  ConnectionId,
} from "../generated/putty_interface_pb.ts";
import { RemoteConnection } from "../generated/putty_interface_connect.ts";

import type { XtermHandle } from "../TerminalPane";

/* ---------- public API types ------------------------------------ */
export type Mode = "serial" | "ssh";
export interface SerialCfg { port: string; baud: number }
export interface SshCfg    { host: string; port: number; user: string; password: string }

/* ---------- gRPC-Web transport ---------------------------------- */
const transport = createGrpcWebTransport({
  baseUrl: window.location.origin,      // :5173 (dev)  | :50051 (prod)
  useBinaryFormat: true,
});
const rpc = createPromiseClient(RemoteConnection, transport);

/* ---------- hook ------------------------------------------------ */
export default function useGrpc() {
  const termRef  = useRef<XtermHandle|null>(null);

  const [connId,     setConnId]     = useState<string|null>(null);
  const [connecting, setConnecting] = useState(false);

  /* install one onKey handler per connection -- avoids double echo */
  const keyDispRef = useRef<import("xterm").IDisposable|null>(null);
  function attachKeyHandler(id: string) {
    keyDispRef.current?.dispose();                 // stale one (if any)
    const term = termRef.current?.term;
    if (!term) return;
    keyDispRef.current = term.onKey(({ key, domEvent }) => {
      domEvent.preventDefault();                   // stop local echo
      rpc.write(new WriteRequest({
        id,
        data: new TextEncoder().encode(key),
      })).catch(console.error);
    });
  }

  /* connect / reconnect ------------------------------------------ */
  async function connect(mode: Mode, cfg: SerialCfg | SshCfg) {
    if (connecting || connId) return;
    setConnecting(true);
    termRef.current?.term.clear();

    /* --- build CreateRequest one-of ----------------------------- */
    const req = new CreateRequest(mode === "serial"
      ? { kind: { case: "serial", value: new Serial(cfg as SerialCfg) } }
      : { kind: { case: "ssh",    value: new Ssh   (cfg as SshCfg   ) } }
    );

    try {
      /* unary RPC ------------------------------------------------ */
      const { id } = await rpc.createRemoteConnection(req) as ConnectionId;
      setConnId(id);
      setConnecting(false);
      attachKeyHandler(id);

      /* async reader (doesn't block UI) -------------------------- */
      (async () => {
        for await (const chunk of rpc.read(new ConnectionId({ id }))) {
          termRef.current?.term.write(chunk.data);
        }
        setConnId(null);                          // stream closed
      })().catch(console.error);

      return id;
    } catch (e) {
      termRef.current?.term.writeln(`\r\n\x1b[31m*** ${e}\x1b[0m\r\n`);
      setConnId(null);
      setConnecting(false);
      throw e;
    }
  }

  /* low-level write helper --------------------------------------- */
  async function write(data: Uint8Array) {
    if (!connId) return;
    await rpc.write(new WriteRequest({ id: connId, data }));
  }

  /* stop the current connection ---------------------------------- */
  function stop() {
    connId && rpc.stop(new ConnectionId({ id: connId })).catch(console.error);
    setConnId(null);
    keyDispRef.current?.dispose();
  }

  return { connId, connecting, connect, write, stop, termRef };
}
