import { forwardRef, useEffect, useImperativeHandle, useRef } from "react";
import { Terminal }  from "xterm";
import { FitAddon }  from "@xterm/addon-fit";
import "xterm/css/xterm.css";

/* shape we expose to parents */
export interface XtermHandle { term: Terminal }

const TerminalPane = forwardRef<XtermHandle>((_props, ref) => {
  const elRef   = useRef<HTMLDivElement>(null);
  const termRef = useRef<Terminal|null>(null);

  useEffect(() => {
    const term = new Terminal({
      fontFamily: "monospace",
      cursorBlink: true,
      scrollback: 10_000,
      theme: { background: "#141414" },
    });
    const fit  = new FitAddon();
    term.loadAddon(fit);

    term.open(elRef.current!);
    fit.fit();
    window.addEventListener("resize", () => fit.fit());

    termRef.current = term;
    return () => term.dispose();
  }, []);

  useImperativeHandle(ref, () => ({
    get term() { return termRef.current!; },
  }), []);

  return <div ref={elRef} style={{ height:"100%", width:"100%" }}/>;
});

export default TerminalPane;
