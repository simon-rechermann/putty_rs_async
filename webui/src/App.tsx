import { useEffect, useRef, useState } from "react";
import { createGrpcWebTransport } from "@connectrpc/connect-web";
import { createPromiseClient }    from "@connectrpc/connect";

import {
  CreateRequest,
  Serial,
  WriteRequest,
  ConnectionId,
  ByteChunk,
} from "./generated/putty_interface_pb.ts";

import { RemoteConnection } from "./generated/putty_interface_connect.ts";

import "./App.css";

/* ------------------------------------------------------------------ */
/*  transport + client                                                */
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
  const [connId, setConnId] = useState<string>();
  const [log,   setLog]     = useState("");
  const inputRef = useRef<HTMLInputElement>(null);

  /* create connection once ---------------------------------------- */
  useEffect(() => {
    (async () => {
      /* one-of has to be set via `kind` ---------------------------- */
      const req = new CreateRequest({
        kind: {
          case: "serial",
          value: new Serial({ port: "/dev/ttyUSB0", baud: 9600 }),
        },
      });

      /* UNARY create → ConnectionId -------------------------------- */
      const res = (await client.createRemoteConnection(req)) as ConnectionId;
      setConnId(res.id);

      /* SERVER-STREAM read ----------------------------------------- */
      for await (const chunk of client.read(new ConnectionId({ id: res.id }))) {
        const text = new TextDecoder().decode((chunk as ByteChunk).data);
        setLog(prev => prev + text);
      }
    })().catch(console.error);
  }, []);

  /* helper to send a line ----------------------------------------- */
  async function send() {
    if (!connId || !inputRef.current) return;
    const data = new TextEncoder().encode(inputRef.current.value + "\n");
    inputRef.current.value = "";
    await client.write(new WriteRequest({ id: connId, data }));
  }

  /* ---------------------------------------------------------------- */
  return (
    <div className="App" style={{ fontFamily: "sans-serif", padding: "1rem" }}>
      <h1>putty-rs (Connect-Web)</h1>

      <input
        ref={inputRef}
        placeholder="type + Enter"
        onKeyDown={e => e.key === "Enter" && send()}
        style={{ padding: ".4rem .6rem", width: "100%" }}
      />

      <pre style={{
        marginTop: "1rem",
        background: "#111", color: "#0f0",
        height: "60vh", overflowY: "auto", whiteSpace: "pre-wrap",
        padding: "1rem",
      }}>
        {log || "connected, waiting…"}
      </pre>
    </div>
  );
}
