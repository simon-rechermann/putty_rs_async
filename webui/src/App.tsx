import { useState }  from "react";
import useGrpc, {
  type Mode, type SerialCfg, type SshCfg,
} from "./components/useGrpc";

import Toolbar       from "./components/Toolbar";
import TerminalPane  from "./TerminalPane";
import ProfilesModal from "./components/ProfilesModal";

import "./App.css";

/* ---------- top-level component -------------------------------- */
export default function App() {
  /* local form state ------------------------------------------- */
  const [mode, setMode]           = useState<Mode>("serial");
  const [serialCfg, setSerialCfg] = useState<SerialCfg>({ port:"/dev/ttyUSB0", baud:115200 });
  const [sshCfg,    setSshCfg]    = useState<SshCfg>   ({ host:"127.0.0.1", port:22,
                                                           user:"user", password:"" });

  /* hook: gRPC connection + terminal handle -------------------- */
  const {
    connId, connecting, connect, termRef,
    listProfiles, saveSerial, saveSsh, deleteProfile, connectProfile,
  } = useGrpc();
  /* modal state for profiles management ----------------------- */
  const [showProfiles, setShowProfiles] = useState(false);

  return (
    <div className="app">
      <Toolbar
        mode={mode} setMode={setMode}
        serialCfg={serialCfg} setSerialCfg={setSerialCfg}
        sshCfg={sshCfg}       setSshCfg={setSshCfg}
        connecting={connecting}
        connected={!!connId}
        connect={connect}
        openProfiles={() => setShowProfiles(true)}
      />

      <div className="term-wrapper">
        <TerminalPane ref={termRef}/>
      </div>
      <ProfilesModal
        show={showProfiles}
        onClose={() => setShowProfiles(false)}
        currentMode={mode}
        currentSerial={serialCfg}
        currentSsh={sshCfg}
        listProfiles={listProfiles}
        saveSerial={saveSerial}
        saveSsh={saveSsh}
        deleteProfile={deleteProfile}
        connectProfile={connectProfile}
      />      
    </div>
  );
}
