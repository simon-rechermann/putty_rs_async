import type { Mode } from "./useGrpc";

type Props = {
  mode: Mode;
  setMode: (m: Mode) => void;

  serialCfg: { port: string; baud: number };
  setSerialCfg: (c: Props["serialCfg"]) => void;

  sshCfg: { host: string; port: number; user: string; password: string };
  setSshCfg: (c: Props["sshCfg"]) => void;

  connecting: boolean;
  connected:  boolean;
  connect: () => void;
};

export default function Toolbar(p: Props) {
  const {
    mode, setMode,
    serialCfg, setSerialCfg,
    sshCfg,    setSshCfg,
    connecting, connected,
    connect,
  } = p;

  return (
    <div className="toolbar">
      <label>Mode&nbsp;
        <select value={mode} onChange={e=>setMode(e.target.value as Mode)}>
          <option value="serial">Serial</option>
          <option value="ssh">SSH</option>
        </select>
      </label>

      {mode==="serial" && (
        <>
          <label>Port&nbsp;
            <input value={serialCfg.port}
                   onChange={e=>setSerialCfg({...serialCfg,port:e.target.value})}/>
          </label>
          <label>Baud&nbsp;
            <input type="number" value={serialCfg.baud}
                   onChange={e=>setSerialCfg({...serialCfg,baud:+e.target.value})}/>
          </label>
        </>
      )}

      {mode==="ssh" && (
        <>
          <label>Host&nbsp;
            <input value={sshCfg.host}
                   onChange={e=>setSshCfg({...sshCfg,host:e.target.value})}/>
          </label>
          <label>Port&nbsp;
            <input type="number" value={sshCfg.port}
                   onChange={e=>setSshCfg({...sshCfg,port:+e.target.value})}/>
          </label>
          <label>User&nbsp;
            <input value={sshCfg.user}
                   onChange={e=>setSshCfg({...sshCfg,user:e.target.value})}/>
          </label>
          <label>Pw&nbsp;
            <input type="password" value={sshCfg.password}
                   onChange={e=>setSshCfg({...sshCfg,password:e.target.value})}/>
          </label>
        </>
      )}

      <button onClick={connect} disabled={connecting||connected}>
        {connecting ? "connecting…" : connected ? "connected" : "connect"}
      </button>
    </div>
  );
}
