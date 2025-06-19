// src/App.tsx
import { useRef, useState } from "react";
import { createGrpcWebTransport } from "@connectrpc/connect-web";
import { createPromiseClient }    from "@connectrpc/connect";

import {
  CreateRequest,
  Serial,
  Ssh,
  WriteRequest,
  ConnectionId,
  ByteChunk,
} from "./generated/putty_interface_pb.ts";
import { RemoteConnection } from "./generated/putty_interface_connect.ts";

import "./App.css";

/* ------------------------------------------------------------------ */
/*  transport + singleton client                                      */
/* ------------------------------------------------------------------ */
const transport = createGrpcWebTransport({
  baseUrl: window.location.origin,
  useBinaryFormat: true,
});
const client = createPromiseClient(RemoteConnection, transport);

/* ------------------------------------------------------------------ */
/*  React component                                                   */
/* ------------------------------------------------------------------ */
export default function App() {
  /* ───── connection-state ───────────────────────────────────────── */
  const [connId, setConnId] = useState<string | null>(null);
  const [log,    setLog]    = useState("");

  /* ───── UI form state ──────────────────────────────────────────── */
  const [mode, setMode] = useState<"serial" | "ssh">("serial");

  const [serialPort, setSerialPort] = useState("/dev/ttyUSB0");
  const [baud,       setBaud]       = useState(115200);

  const [sshHost,     setSshHost]     = useState("127.0.0.1");
  const [sshPort,     setSshPort]     = useState(22);
  const [sshUser,     setSshUser]     = useState("user");
  const [sshPassword, setSshPassword] = useState("");

  const [connecting, setConnecting] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  /* ───── (re)connect ────────────────────────────────────────────── */
  async function connect() {
    if (connecting || connId) return;
    setConnecting(true);
    setLog("");

    /* build protobuf CreateRequest -------------------------------- */
    let req: CreateRequest;
    if (mode === "serial") {
      req = new CreateRequest({
        kind: {
          case: "serial",
          value: new Serial({ port: serialPort, baud }),
        },
      });
    } else {
      req = new CreateRequest({
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
    }

    try {
      /* UNARY create → ConnectionId ------------------------------- */
      const res = await client.createRemoteConnection(req) as ConnectionId;
      setConnId(res.id);

      /* SERVER-STREAM read (runs until server closes / Stop) ------- */
      for await (const chunk of client.read(new ConnectionId({ id: res.id }))) {
        const text = new TextDecoder().decode((chunk as ByteChunk).data);
        setLog(prev => prev + text);
      }
      setConnId(null);              // stream ended
    } catch (e) {
      console.error(e);
      setLog(String(e));
      setConnId(null);
    } finally {
      setConnecting(false);
    }
  }

  /* ───── write helper ───────────────────────────────────────────── */
  async function send() {
    if (!connId || !inputRef.current) return;
    const data = new TextEncoder().encode(inputRef.current.value + "\n");
    inputRef.current.value = "";
    await client.write(new WriteRequest({ id: connId, data }));
  }

  /* ───── render ─────────────────────────────────────────────────── */
  return (
    <div className="App" style={{ fontFamily: "sans-serif", padding: "1rem", maxWidth: 640 }}>
      <h1>putty-rs (gRPC-Web)</h1>

      {/* mode selector ------------------------------------------------ */}
      <label style={{ fontWeight: "bold" }}>Mode:&nbsp;</label>
      <select value={mode} onChange={e => setMode(e.target.value as any)}>
        <option value="serial">Serial</option>
        <option value="ssh">SSH</option>
      </select>

      {/* serial cfg --------------------------------------------------- */}
      {mode === "serial" && (
        <div style={{ marginTop: 8 }}>
          <label>
            Port:&nbsp;
            <input value={serialPort} onChange={e => setSerialPort(e.target.value)} />
          </label>
          &nbsp;&nbsp;
          <label>
            Baud:&nbsp;
            <input type="number" value={baud} onChange={e => setBaud(Number(e.target.value))} />
          </label>
        </div>
      )}

      {/* ssh cfg ------------------------------------------------------ */}
      {mode === "ssh" && (
        <div style={{ marginTop: 8, display: "grid", gridTemplateColumns: "auto 1fr", gap: 4 }}>
          <label>Host:</label>      <input value={sshHost}     onChange={e => setSshHost(e.target.value)} />
          <label>Port:</label>      <input type="number" value={sshPort}
                                          onChange={e => setSshPort(Number(e.target.value))} />
          <label>User:</label>      <input value={sshUser}     onChange={e => setSshUser(e.target.value)} />
          <label>Password:</label>  <input type="password" value={sshPassword}
                                          onChange={e => setSshPassword(e.target.value)} />
        </div>
      )}

      {/* connect / status -------------------------------------------- */}
      <button
        onClick={connect}
        disabled={connecting || !!connId}
        style={{ marginTop: 12, padding: ".4rem 1rem" }}
      >
        {connecting ? "connecting…" : connId ? "connected" : "connect"}
      </button>

      {/* input -------------------------------------------------------- */}
      <input
        ref={inputRef}
        placeholder="type & hit Enter"
        onKeyDown={e => e.key === "Enter" && send()}
        disabled={!connId}
        style={{ marginTop: 12, padding: ".4rem .6rem", width: "100%" }}
      />

      {/* terminal ----------------------------------------------------- */}
      <pre style={{
        marginTop: 12,
        background: "#111", color: "#0f0",
        height: "50vh", overflowY: "auto", whiteSpace: "pre-wrap",
        padding: "1rem",
      }}>
        {log || (connId ? "connected, waiting…" : "not connected")}
      </pre>
    </div>
  );
}
