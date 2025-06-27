import { useRef, useState } from "react";
import { createGrpcWebTransport } from "@connectrpc/connect-web";
import { createClient }    from "@connectrpc/connect";

import {
  CreateRequest, Serial, Ssh, WriteRequest,
  ConnectionId, ProfileReq, ProfileName, Empty,
} from "../generated/putty_interface_pb.ts";
import { RemoteConnection } from "../generated/putty_interface_connect.ts";

import type { XtermHandle } from "../TerminalPane";

/* ---------- public API types ------------------------------------ */
export type Mode = "serial" | "ssh";
export interface SerialCfg { port: string; baud: number }
export interface SshCfg    { host: string; port: number; user: string; password: string }
export type ListProfilesFn   = () => Promise<ProfileReq[]>;
export type SaveSerialFn     = (name:string,cfg:SerialCfg)=>Promise<void>;
export type SaveSshFn        = (name:string,cfg:SshCfg)   =>Promise<void>;
export type DeleteProfileFn  = (name:string)=>Promise<void>;
export type ConnectProfileFn = (name: string) => Promise<string | undefined>;

/* ---------- gRPC-Web transport ---------------------------------- */
const transport = createGrpcWebTransport({
  baseUrl: window.location.origin,      // :5173 (dev)  | :50051 (prod)
  useBinaryFormat: true,
});
const rpc = createClient(RemoteConnection, transport);

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
    const req = new CreateRequest(mode === "serial"
      ? { kind: { case: "serial", value: new Serial(cfg as SerialCfg) } }
      : { kind: { case: "ssh",    value: new Ssh   (cfg as SshCfg   ) } }
    );
    return openRequest(req);
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

  async function listProfiles() {
    const { profiles } = await rpc.listProfiles(new Empty());
    return profiles;
  }

  async function saveSerial(name:string,cfg:SerialCfg){
    await rpc.saveProfile(new ProfileReq({
      name,
      kind:{ case:"serial", value:new Serial(cfg) }
    }));
  }

  async function saveSsh(name:string,cfg:SshCfg){
    await rpc.saveProfile(new ProfileReq({
      name,
      kind:{ case:"ssh", value:new Ssh(cfg) }
    }));
  }

  async function deleteProfile(name:string){
    await rpc.deleteProfile(new ConnectionId({ id:name }));
  }

  async function connectProfile(name: string) {
    const req = new CreateRequest({
      kind: { case: "profile", value: new ProfileName({ name }) },
    });
    return openRequest(req);
  }

  async function openRequest(req: CreateRequest) {
    if (connecting || connId) return;
    setConnecting(true);
    termRef.current?.term.clear();

    try {
      const { id } = await rpc.createRemoteConnection(req) as ConnectionId;
      setConnId(id);
      setConnecting(false);
      attachKeyHandler(id);

      /* start reader --------------------------------------------------- */
      (async () => {
        for await (const chunk of rpc.read(new ConnectionId({ id }))) {
          termRef.current?.term.write(chunk.data);
        }
        setConnId(null);                      // server closed stream
      })().catch(console.error);

      return id;
    } catch (e) {
      termRef.current?.term.writeln(`\r\n\x1b[31m*** ${e}\x1b[0m\r\n`);
      setConnId(null);
      setConnecting(false);
      throw e;
    }
  }

return {
  connId, connecting, connect, write, stop, termRef,
  listProfiles, saveSerial, saveSsh, deleteProfile, connectProfile,
};
}
