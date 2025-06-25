import { useRef, useState } from "react";
import { createGrpcWebTransport } from "@connectrpc/connect-web";
import { createPromiseClient }    from "@connectrpc/connect";

import {
  CreateRequest, Serial, Ssh, WriteRequest, ConnectionId,
} from "../generated/putty_interface_pb";
import { RemoteConnection }       from "../generated/putty_interface_connect";
import type { XtermHandle }       from "../TerminalPane";

/* -------- transport + singleton client -------------------------- */
const transport = createGrpcWebTransport({
  baseUrl: window.location.origin,
  useBinaryFormat: true,
});
const rpc = createPromiseClient(RemoteConnection, transport);

/* ---------------------------------------------------------------- */
export type Mode = "serial" | "ssh";

export default function useGrpc() {
  /* --- UI config --- */
  const [mode, setMode]           = useState<Mode>("serial");
  const [serialCfg, setSerialCfg] = useState({port:"/dev/ttyUSB0", baud:115200});
  const [sshCfg,    setSshCfg]    = useState({
    host:"127.0.0.1", port:22, user:"user", password:""
  });

  /* --- connection --- */
  const [connId, setConnId]       = useState<string|null>(null);
  const [connecting, setConnecting] = useState(false);

  /* --- xterm ref --- */
  const termRef = useRef<XtermHandle|null>(null);

  /* send key-presses to server */
  termRef.current?.term.onData(d =>
    connId && rpc.write(new WriteRequest({
      id: connId,
      data: new TextEncoder().encode(d),
    })).catch(console.error));

  async function connect() {
    if (connecting || connId) return;
    setConnecting(true);
    termRef.current?.term.clear();

    /* build request */
    const req: CreateRequest = mode==="serial"
      ? new CreateRequest({ kind:{case:"serial",
            value:new Serial({port:serialCfg.port, baud:serialCfg.baud})}})
      : new CreateRequest({ kind:{case:"ssh",
            value:new Ssh({...sshCfg})}});

    try {
      const {id} = await rpc.createRemoteConnection(req) as ConnectionId;
      setConnId(id);

      for await (const chunk of rpc.read(new ConnectionId({id}))) {
        termRef.current?.term.write(chunk.data);
      }
    } catch (e){
      termRef.current?.term.writeln(`\r\n\x1b[31m*** ${e}\x1b[0m\r\n`);
    } finally {
      setConnId(null);
      setConnecting(false);
    }
  }

  function stop() {
    if (!connId) return;
    rpc.stop(new ConnectionId({id:connId})).catch(console.error);
    setConnId(null);
  }

  return {
    mode, setMode,
    serialCfg, setSerialCfg,
    sshCfg,    setSshCfg,

    connecting,
    connected: !!connId,
    connect,
    stop,

    termRef,
  };
}
