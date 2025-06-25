import { forwardRef, useImperativeHandle, useRef } from "react";
import { Terminal }  from "xterm";
import { FitAddon }  from "@xterm/addon-fit";
import "xterm/css/xterm.css";

export type XtermHandle = {
  term: Terminal;
  fit:  () => void;
};

const TerminalPane = forwardRef<XtermHandle>((_, ref) => {
  const containerRef   = useRef<HTMLDivElement>(null);
  const termRef        = useRef<Terminal | null>(null);
  const fitAddon       = useRef<FitAddon>(new FitAddon());

  if (!termRef.current) {
    termRef.current = new Terminal({
      fontFamily:"monospace",
      theme:{background:"#1e1e1e"},
      cursorBlink:true,
      scrollback:10_000,
    });
    termRef.current.loadAddon(fitAddon.current);
  }

  useImperativeHandle(ref, () => ({
    term: termRef.current!,
    fit : () => fitAddon.current.fit(),
  }), []);

  return <div ref={el=>{
            if (el && containerRef.current!==el) {
              containerRef.current = el;
              termRef.current!.open(el);
              fitAddon.current.fit();
            }
          }}
          className="term-wrapper"/>;
});

export default TerminalPane;
