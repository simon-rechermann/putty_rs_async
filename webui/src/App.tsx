import { useEffect, useRef, useState }   from "react";
import { createGrpcWebTransport }        from "@connectrpc/connect-web";
import { createPromiseClient }           from "@connectrpc/connect";

import {
  CreateRequest, Serial, Ssh,
  WriteRequest, ConnectionId,
} from "./generated/putty_interface_pb.ts";
import { RemoteConnection }              from "./generated/putty_interface_connect.ts";

/* xterm */
import { Terminal }  from "xterm";
import { FitAddon }  from "@xterm/addon-fit";
import "xterm/css/xterm.css";

/* ------------------------------------------------------------------ */
/*  gRPC-Web transport + singleton client                             */
/* ------------------------------------------------------------------ */
const transport = createGrpcWebTransport({
  baseUrl: window.location.origin,  /* dev :5173 | prod :50051        */
  useBinaryFormat: true,
});
const rpc = createPromiseClient(RemoteConnection, transport);

/* ------------------------------------------------------------------ */
/*  React component                                                   */
/* ------------------------------------------------------------------ */
export default function App() {
  /* connection ----------------------------------------------------- */
  const [connId,      setConnId]      = useState<string | null>(null);
  const [connecting,  setConnecting]  = useState(false);

  /* mode + config -------------------------------------------------- */
  const [mode, setMode]               = useState<"serial" | "ssh">("serial");
  const [serialPort, setSerialPort]   = useState("/dev/ttyUSB0");
  const [baud,       setBaud]         = useState(115200);

  const [sshHost,setSshHost]          = useState("127.0.0.1");
  const [sshPort,setSshPort]          = useState(22);
  const [sshUser,setSshUser]          = useState("user");
  const [sshPassword,setSshPassword]  = useState("");

  /* xterm refs ----------------------------------------------------- */
  const termRef = useRef<Terminal | null>(null);
  const divRef  = useRef<HTMLDivElement | null>(null);

  /* mount xterm once ---------------------------------------------- */
  useEffect(() => {
    const term = new Terminal({
      fontFamily: "monospace",
      cursorBlink: true,
      scrollback: 10_000,
      theme: { background: "#1e1e1e" },
    });
    const fit = new FitAddon();
    term.loadAddon(fit);
    term.open(divRef.current!);
    fit.fit();

    /* keep fitting on resize */
    const onResize = () => fit.fit();
    window.addEventListener("resize", onResize);

    termRef.current = term;
    return () => window.removeEventListener("resize", onResize);
  }, []);

  /* key-press → rpc.write ------------------------------------------ */
  useEffect(() => {
    const term = termRef.current;
    if (!term) return;

    const sub = term.onData(data =>
      connId &&
      rpc.write(new WriteRequest({ id: connId, data: new TextEncoder().encode(data) }))
        .catch(console.error)
    );
    return () => sub.dispose();
  }, [connId]);

  /* connect (or reconnect) ----------------------------------------- */
  async function connect() {
    if (connecting || connId) return;
    setConnecting(true);
    termRef.current?.clear();

    const req =
      mode === "serial"
        ? new CreateRequest({
            kind: { case: "serial", value: new Serial({ port: serialPort, baud }) },
          })
        : new CreateRequest({
            kind: {
              case: "ssh",
              value: new Ssh({
                host: sshHost,
                port: sshPort,
                user: sshUser,
                password: sshPassword,
              }),
            },
          });

    try {
      const { id } = (await rpc.createRemoteConnection(req)) as ConnectionId;
      setConnId(id);

      /* stream → terminal */
      for await (const chunk of rpc.read(new ConnectionId({ id }))) {
        termRef.current?.write(chunk.data);
        termRef.current?.scrollToBottom();           /* auto-follow */
      }
    } catch (e) {
      termRef.current?.writeln(`\r\n\x1b[31m*** ${e}\x1b[0m\r\n`);
    } finally {
      setConnId(null);
      setConnecting(false);
    }
  }

  /* ---------------------------------------------------------------- */
  return (
    <div className="App">
      <h2>putty-rs (gRPC-Web)</h2>

      {/* ─── mode selector ───────────────────────────────────────── */}
      <label>Mode:&nbsp;</label>
      <select value={mode} onChange={e => setMode(e.target.value as any)}>
        <option value="serial">Serial</option>
        <option value="ssh">SSH</option>
      </select>

      {/* ─── config fields ───────────────────────────────────────── */}
      {mode === "serial" && (
        <>
          &nbsp;Port&nbsp;
          <input value={serialPort} onChange={e => setSerialPort(e.target.value)} />
          &nbsp;Baud&nbsp;
          <input
            type="number"
            value={baud}
            onChange={e => setBaud(Number(e.target.value))}
            style={{ width: 80 }}
          />
        </>
      )}

      {mode === "ssh" && (
        <>
          &nbsp;Host&nbsp;
          <input value={sshHost} onChange={e => setSshHost(e.target.value)} />
          &nbsp;Port&nbsp;
          <input
            type="number"
            value={sshPort}
            onChange={e => setSshPort(Number(e.target.value))}
            style={{ width: 70 }}
          />
          &nbsp;User&nbsp;
          <input value={sshUser} onChange={e => setSshUser(e.target.value)} />
          &nbsp;Pw&nbsp;
          <input
            type="password"
            value={sshPassword}
            onChange={e => setSshPassword(e.target.value)}
            style={{ width: 110 }}
          />
        </>
      )}

      <button
        style={{ marginLeft: 12 }}
        disabled={connecting || !!connId}
        onClick={connect}
      >
        {connecting ? "connecting…" : connId ? "connected" : "connect"}
      </button>

      {/* ─── terminal ────────────────────────────────────────────── */}
      <div ref={divRef} className="term-wrapper" />
    </div>
  );
}
