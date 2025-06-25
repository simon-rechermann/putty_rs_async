import { useState }  from "react";
import useGrpc, {
  type Mode, type SerialCfg, type SshCfg,
} from "./components/useGrpc";

import Toolbar       from "./components/Toolbar";
import TerminalPane  from "./TerminalPane";

import "./App.css";

/* ---------- top-level component -------------------------------- */
export default function App() {
  /* local form state ------------------------------------------- */
  const [mode, setMode]           = useState<Mode>("serial");
  const [serialCfg, setSerialCfg] = useState<SerialCfg>({ port:"/dev/ttyUSB0", baud:115200 });
  const [sshCfg,    setSshCfg]    = useState<SshCfg>   ({ host:"127.0.0.1", port:22,
                                                           user:"user", password:"" });

  /* hook: gRPC connection + terminal handle -------------------- */
  const { connId, connecting, connect, termRef }
          = useGrpc();

  return (
    <div className="app">
      <Toolbar
        mode={mode} setMode={setMode}
        serialCfg={serialCfg} setSerialCfg={setSerialCfg}
        sshCfg={sshCfg}       setSshCfg={setSshCfg}
        connecting={connecting}
        connected={!!connId}
        connect={connect}
      />

      <div className="term-wrapper">
        <TerminalPane ref={termRef}/>
      </div>
    </div>
  );
}
