import { useEffect } from "react";
import useGrpc                        from "./components/useGrpc";
import TerminalPane, { type XtermHandle } from "./TerminalPane";  // <-- `type` import

import Toolbar from "./components/Toolbar";
import "./App.css";

export default function App() {
  const {
    mode, setMode,
    serialCfg, setSerialCfg,
    sshCfg,    setSshCfg,
    connecting, connected,
    connect,
    termRef,
  } = useGrpc();

  /* fit xterm whenever size changes */
  useEffect(() => {
    const fit = termRef.current?.fit;
    if (!fit) return;
    fit();
    window.addEventListener("resize", fit as any);
    return () => window.removeEventListener("resize", fit as any);
  }, [termRef]);

  return (
    <div className="app">
      <h2>putty-rs&nbsp;<small style={{fontWeight:400}}>(gRPC-Web)</small></h2>

      <Toolbar
        {...{
          mode, setMode,
          serialCfg, setSerialCfg,
          sshCfg,    setSshCfg,
          connecting, connected,
          connect,
        }}
      />

      <TerminalPane ref={termRef as React.Ref<XtermHandle>} />
    </div>
  );
}
