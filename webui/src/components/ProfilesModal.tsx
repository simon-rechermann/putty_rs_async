import { useEffect, useState } from "react";
import "./ProfilesModal.css";

import type { Mode, SerialCfg, SshCfg } from "./useGrpc";
import type { ProfileReq }             from "../generated/putty_interface_pb";
import type { ListProfilesFn,
              SaveSerialFn, SaveSshFn,
              DeleteProfileFn,
              ConnectProfileFn } from "./useGrpc";

/* ---------------- props ------------------------------------------- */
interface Props {
  show: boolean;
  onClose: () => void;

  /* data from parent */
  currentMode: Mode;
  currentSerial: SerialCfg;
  currentSsh: SshCfg;

  /* RPC helpers from useGrpc() */
  listProfiles: ListProfilesFn;
  saveSerial:  SaveSerialFn;
  saveSsh:     SaveSshFn;
  deleteProfile: DeleteProfileFn;
  connectProfile: ConnectProfileFn;
}

/* ---------------- implementation ---------------------------------- */
export default function ProfilesModal(p: Props) {
  const {
    show, onClose,
    currentMode, currentSerial, currentSsh,
    listProfiles, saveSerial, saveSsh, deleteProfile, connectProfile,
  } = p;

  const [profiles, setProfiles]   = useState<ProfileReq[]>([]);
  const [loading,  setLoading]    = useState(false);
  const [error,    setError]      = useState<string|null>(null);
  const [newName,  setNewName]    = useState("");

  /* load list each time modal opens -------------------------------- */
  useEffect(()=>{
    if (!show) return;
    setLoading(true);
    listProfiles()
      .then(setProfiles)
      .catch(e=>setError(String(e)))
      .finally(()=>setLoading(false));
  },[show]);

  if (!show) return null;          // nothing rendered

  /* -- helpers ----------------------------------------------------- */
  async function handleSave() {
    try {
      if (!newName.trim()) return;
      if (currentMode==="serial") {
        await saveSerial(newName.trim(), currentSerial);
      } else {
        await saveSsh(newName.trim(), currentSsh);
      }
      setNewName("");
      const lst = await listProfiles();
      setProfiles(lst);
    } catch(e){ setError(String(e)); }
  }

  async function handleApply(name:string){
    try{
      await connectProfile(name);
      onClose();                // close after successful connect
    }catch(e){setError(String(e));}
  }
  async function handleDelete(name:string){
    try{
      await deleteProfile(name);
      setProfiles(await listProfiles());
    }catch(e){setError(String(e));}
  }

  /* -- JSX  -------------------------------------------------------- */
  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" onClick={e=>e.stopPropagation()}>
        <h2>Profiles</h2>

        {loading && <p>loading…</p>}
        {error   && <p className="error">{error}</p>}

        {/* list ---------------------------------------------------- */}
        <table className="profile-table">
          <thead><tr><th>Name</th><th>Kind</th><th></th></tr></thead>
          <tbody>
            {profiles.map(p=><tr key={p.name}>
              <td>{p.name}</td>
              <td>{"serial" in p.kind! ? "Serial" : "SSH"}</td>
              <td className="actions">
                <button onClick={()=>handleApply(p.name!)}>use</button>
                <button onClick={()=>handleDelete(p.name!)}>del</button>
              </td>
            </tr>)}
          </tbody>
        </table>

        {/* save current form -------------------------------------- */}
        <h3>Save current {currentMode} settings as…</h3>
        <input value={newName} placeholder="profile name"
               onChange={e=>setNewName(e.target.value)}/>
        <button onClick={handleSave}>save</button>

        <button className="close-btn" onClick={onClose}>close</button>
      </div>
    </div>
  );
}
